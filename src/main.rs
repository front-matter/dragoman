use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::signal::unix::{SignalKind, signal};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod db;
mod error;
mod negotiate;

use error::AppError;
use negotiate::NegotiateResult;

// ── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "dragoman", about = "PID redirection and content negotiation server")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the server (runs in the foreground).
    Start(StartArgs),
    /// Stop a running server by sending SIGTERM to its PID file.
    Stop(StopArgs),
}

#[derive(Args)]
struct StartArgs {
    /// TCP port to listen on.
    #[arg(short, long, env = "PORT", default_value_t = 3456)]
    port: u16,

    /// Path to a local commonmeta SQLite3 database. When set, metadata is
    /// served from the database before falling back to the live API.
    #[arg(short, long, env = "DRAGOMAN_DB")]
    db: Option<PathBuf>,

    /// Write the server PID to this file on startup so that `dragoman stop`
    /// can find and terminate the process.
    #[arg(long, env = "DRAGOMAN_PID_FILE")]
    pid_file: Option<PathBuf>,
}

#[derive(Args)]
struct StopArgs {
    /// PID file written by `dragoman start --pid-file`.
    #[arg(long, env = "DRAGOMAN_PID_FILE", default_value = "/tmp/dragoman.pid")]
    pid_file: PathBuf,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dragoman=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Start(args) => cmd_start(args).await,
        Command::Stop(args) => cmd_stop(args),
    }
}

// ── `start` ──────────────────────────────────────────────────────────────────

async fn cmd_start(args: StartArgs) {
    let database = match args.db {
        None => None,
        Some(ref path) => match db::open(path).await {
            Ok(d) => {
                tracing::info!(path = %path.display(), "using local sqlite database");
                Some(Arc::new(d))
            }
            Err(e) => {
                tracing::error!(path = %path.display(), error = %e, "failed to open database");
                std::process::exit(1);
            }
        },
    };

    if let Some(ref path) = args.pid_file {
        if let Err(e) = std::fs::write(path, std::process::id().to_string()) {
            tracing::error!(path = %path.display(), error = %e, "failed to write PID file");
            std::process::exit(1);
        }
        tracing::info!(path = %path.display(), "wrote PID file");
    }

    let app = build_app(AppState { db: database });

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(port = args.port, error = %e, "failed to bind");
            if let Some(ref path) = args.pid_file {
                std::fs::remove_file(path).ok();
            }
            std::process::exit(1);
        }
    };
    tracing::info!("listening on {addr}");

    let pid_file = args.pid_file.clone();
    let shutdown = async move {
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, "failed to register SIGTERM handler");
                std::process::exit(1);
            }
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
        if let Some(ref path) = pid_file {
            std::fs::remove_file(path).ok();
            tracing::info!(path = %path.display(), "removed PID file");
        }
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        tracing::error!(error = %e, "server error");
        std::process::exit(1);
    }
}

// ── `stop` ────────────────────────────────────────────────────────────────────

fn cmd_stop(args: StopArgs) {
    let pid_str = match std::fs::read_to_string(&args.pid_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "error: cannot read PID file '{}': {e}",
                args.pid_file.display()
            );
            std::process::exit(1);
        }
    };

    let pid: u32 = match pid_str.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            eprintln!(
                "error: PID file '{}' does not contain a valid PID",
                args.pid_file.display()
            );
            std::process::exit(1);
        }
    };

    let status = std::process::Command::new("kill")
        .arg(pid.to_string())
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("sent SIGTERM to process {pid}");
            std::fs::remove_file(&args.pid_file).ok();
        }
        Ok(_) => {
            eprintln!("error: kill({pid}) failed — process may have already exited");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: failed to run kill: {e}");
            std::process::exit(1);
        }
    }
}

// ── App state & query params ─────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    db: Option<Arc<libsql::Database>>,
}

#[derive(Deserialize)]
struct PidQuery {
    /// Override content negotiation: named commonmeta format (e.g. `bibtex`, `ris`, `csl`).
    format: Option<String>,
    /// Override registration agency: `crossref` or `datacite`.
    source: Option<String>,
    /// Citation style name (CSL style repo, e.g. `apa`). Only for `citation` format.
    style: Option<String>,
    /// Citation locale (e.g. `fr-FR`). Only for `citation` format.
    locale: Option<String>,
}

// ── HTTP handlers ─────────────────────────────────────────────────────────────

async fn index() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../ui/dist/index.html"),
    )
}

async fn handle_pid(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<PidQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let doi = commonmeta::doi_utils::validate_doi(&path)
        .ok_or_else(|| AppError::NotFound(path.clone()))?;

    tracing::info!(doi = %doi, "resolving PID");

    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/html");

    let (format, content_type, style, locale): (
        String,
        &'static str,
        Option<String>,
        Option<String>,
    ) = if let Some(fmt) = &query.format {
        let ct = negotiate::content_type_for_format(fmt)
            .ok_or_else(|| AppError::UnsupportedFormat(fmt.clone()))?;
        (fmt.clone(), ct, query.style.clone(), query.locale.clone())
    } else {
        match negotiate::negotiate(accept) {
            NegotiateResult::Format(n) => (
                n.format.to_string(),
                n.content_type,
                n.style.or_else(|| query.style.clone()),
                n.locale.or_else(|| query.locale.clone()),
            ),
            NegotiateResult::Redirect => {
                let url = resolve_url(&doi, query.source.as_deref(), state.db.as_deref()).await?;
                return Ok(Redirect::temporary(&url).into_response());
            }
            NegotiateResult::NotAcceptable => {
                return Err(AppError::UnsupportedFormat(accept.to_string()));
            }
        }
    };

    let bytes = fetch_and_convert(
        &doi,
        &format,
        query.source.as_deref(),
        style.as_deref(),
        locale.as_deref(),
        state.db.as_deref(),
    )
    .await?;

    Ok((StatusCode::OK, [(header::CONTENT_TYPE, content_type)], bytes).into_response())
}

// ── Business logic ────────────────────────────────────────────────────────────

async fn fetch_and_convert(
    doi: &str,
    format: &str,
    source: Option<&str>,
    style: Option<&str>,
    locale: Option<&str>,
    db: Option<&libsql::Database>,
) -> Result<Vec<u8>, AppError> {
    if let Some(database) = db {
        if let Some(data) = db::lookup(database, doi).await? {
            tracing::debug!(doi = %doi, "served from local sqlite");
            let format = format.to_string();
            let style = style.map(str::to_string);
            let locale = locale.map(str::to_string);
            return tokio::task::spawn_blocking(move || {
                commonmeta::write_with_style(&format, &data, style.as_deref(), locale.as_deref())
                    .map_err(|e| AppError::FetchError(e.to_string()))
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }

    let doi = doi.to_string();
    let format = format.to_string();
    let source = source.map(str::to_string);
    let style = style.map(str::to_string);
    let locale = locale.map(str::to_string);

    tokio::task::spawn_blocking(move || {
        let from = resolve_source(&doi, source.as_deref());
        if format == "citation" {
            commonmeta::convert_citation(&from, &doi, style.as_deref(), locale.as_deref())
                .map_err(|e| AppError::FetchError(e.to_string()))
        } else {
            commonmeta::convert(&from, &format, &doi)
                .map_err(|e| AppError::FetchError(e.to_string()))
        }
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

async fn resolve_url(
    doi: &str,
    source: Option<&str>,
    db: Option<&libsql::Database>,
) -> Result<String, AppError> {
    if let Some(database) = db {
        if let Some(data) = db::lookup(database, doi).await? {
            if !data.url.is_empty() {
                tracing::debug!(doi = %doi, "url served from local sqlite");
                return Ok(data.url);
            }
        }
    }

    let doi = doi.to_string();
    let source = source.map(str::to_string);

    tokio::task::spawn_blocking(move || {
        let from = resolve_source(&doi, source.as_deref());
        let data = commonmeta::read(&from, &doi)
            .map_err(|e| AppError::NotFound(e.to_string()))?;
        if data.url.is_empty() {
            Err(AppError::NotFound(doi))
        } else {
            Ok(data.url)
        }
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

// ── /bibliography ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct BibliographyRequest {
    items: Vec<BibRequestItem>,
    style: Option<String>,
    locale: Option<String>,
}

#[derive(Deserialize)]
struct BibRequestItem {
    id: String,
    data: String, // commonmeta JSON string
}

#[derive(Serialize)]
struct BibliographyResponse {
    items: Vec<BibResponseItem>,
}

#[derive(Serialize)]
struct BibResponseItem {
    id: String,
    html: String,
}

async fn handle_bibliography(
    Json(req): Json<BibliographyRequest>,
) -> Result<Json<BibliographyResponse>, AppError> {
    let style = req.style;
    let locale = req.locale;
    let items = req.items;

    let results = tokio::task::spawn_blocking(move || {
        items
            .into_iter()
            .map(|item| {
                let data = commonmeta::read("commonmeta", &item.data)
                    .map_err(|e| AppError::FetchError(e.to_string()))?;
                let bytes = commonmeta::write_with_style(
                    "citation",
                    &data,
                    style.as_deref(),
                    locale.as_deref(),
                )
                .map_err(|e| AppError::FetchError(e.to_string()))?;
                Ok(BibResponseItem {
                    id: item.id,
                    html: String::from_utf8_lossy(&bytes).into_owned(),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))??;

    Ok(Json(BibliographyResponse { items: results }))
}

fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/bibliography", post(handle_bibliography))
        .route("/{*path}", get(handle_pid))
        .with_state(state)
}

fn resolve_source(doi: &str, source: Option<&str>) -> String {
    if let Some(s) = source {
        return s.to_lowercase();
    }
    match commonmeta::doi_utils::get_doi_ra_sync(doi)
        .as_deref()
        .map(str::to_lowercase)
        .as_deref()
    {
        Some("crossref") => "crossref".to_string(),
        Some("datacite") => "datacite".to_string(),
        _ => "crossref".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt; // for .oneshot()

    /// Test database DDL — mirrors the commonmeta `works` schema.
    const TEST_DDL: &str = r#"
        CREATE TABLE works (
            "id"                TEXT PRIMARY KEY NOT NULL,
            "type"              TEXT NOT NULL DEFAULT '',
            "url"               TEXT NOT NULL DEFAULT '',
            "title"             TEXT NOT NULL DEFAULT '',
            "additional_titles" TEXT NOT NULL DEFAULT '[]',
            "contributors"      TEXT NOT NULL DEFAULT '[]',
            "date_published"    TEXT NOT NULL DEFAULT '',
            "date_updated"      TEXT NOT NULL DEFAULT '',
            "dates"             TEXT NOT NULL DEFAULT '{}',
            "publisher"         TEXT NOT NULL DEFAULT '{}',
            "container"         TEXT NOT NULL DEFAULT '{}',
            "description"       TEXT NOT NULL DEFAULT '',
            "license"           TEXT NOT NULL DEFAULT '{}',
            "version"           TEXT NOT NULL DEFAULT '',
            "language"          TEXT NOT NULL DEFAULT '',
            "subjects"          TEXT NOT NULL DEFAULT '[]',
            "identifiers"       TEXT NOT NULL DEFAULT '[]',
            "relations"         TEXT NOT NULL DEFAULT '[]',
            "references"        TEXT NOT NULL DEFAULT '[]',
            "funding_references" TEXT NOT NULL DEFAULT '[]',
            "geo_locations"     TEXT NOT NULL DEFAULT '[]',
            "files"             TEXT NOT NULL DEFAULT '[]',
            "archive_locations" TEXT NOT NULL DEFAULT '[]',
            "provider"          TEXT NOT NULL DEFAULT ''
        )
    "#;

    async fn make_test_db() -> (tempfile::TempDir, Arc<libsql::Database>) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let db = libsql::Builder::new_local(&path)
            .build()
            .await
            .unwrap();
        let conn = db.connect().unwrap();
        conn.execute_batch(TEST_DDL).await.unwrap();
        conn.execute(
            r#"INSERT INTO works ("id", "type", "url", "title", "contributors", "date_published", "provider")
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            libsql::params![
                "https://doi.org/10.1234/test",
                "JournalArticle",
                "https://example.com/test-article",
                "Test Article on Content Negotiation",
                r#"[{"name": "Doe, Jane", "contributorRoles": ["Author"]}]"#,
                "2024-01-15",
                "Crossref"
            ],
        )
        .await
        .unwrap();
        (dir, Arc::new(db))
    }

    fn app_no_db() -> Router {
        build_app(AppState { db: None })
    }

    async fn app_with_db() -> (tempfile::TempDir, Router) {
        let (dir, db) = make_test_db().await;
        let app = build_app(AppState { db: Some(db) });
        (dir, app)
    }

    // ── Index ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn index_returns_200() {
        let response = app_no_db()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ── 404 for non-DOI paths ─────────────────────────────────────────────────

    #[tokio::test]
    async fn non_doi_path_returns_404() {
        let response = app_no_db()
            .oneshot(
                Request::builder()
                    .uri("/not-a-doi")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn arbitrary_text_path_returns_404() {
        let response = app_no_db()
            .oneshot(
                Request::builder()
                    .uri("/hello/world")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ── 406 for unsupported Accept types ──────────────────────────────────────

    #[tokio::test]
    async fn unsupported_accept_returns_406() {
        let response = app_no_db()
            .oneshot(
                Request::builder()
                    .uri("/10.1234/test")
                    .header("Accept", "application/rdf+xml")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);
    }

    #[tokio::test]
    async fn multiple_unsupported_accept_returns_406() {
        let response = app_no_db()
            .oneshot(
                Request::builder()
                    .uri("/10.1234/test")
                    .header("Accept", "application/rdf+xml, image/png")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);
    }

    #[tokio::test]
    async fn invalid_format_query_param_returns_406() {
        let response = app_no_db()
            .oneshot(
                Request::builder()
                    .uri("/10.1234/test?format=nonsense")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_ACCEPTABLE);
    }

    // ── DB-backed responses (no live API call) ────────────────────────────────

    #[tokio::test]
    async fn html_accept_with_db_returns_redirect() {
        let (_dir, app) = app_with_db().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/10.1234/test")
                    .header("Accept", "text/html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        assert_eq!(location, "https://example.com/test-article");
    }

    #[tokio::test]
    async fn bibtex_accept_with_db_returns_200() {
        let (_dir, app) = app_with_db().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/10.1234/test")
                    .header("Accept", "application/x-bibtex")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let ct = response.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/x-bibtex"), "unexpected content-type: {ct}");
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        assert!(text.starts_with('@'), "bibtex should start with '@': {text}");
    }

    #[tokio::test]
    async fn format_query_param_with_db_returns_bibtex() {
        let (_dir, app) = app_with_db().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/10.1234/test?format=bibtex")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let ct = response.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/x-bibtex"), "unexpected content-type: {ct}");
    }

    #[tokio::test]
    async fn csl_json_accept_with_db_returns_200() {
        let (_dir, app) = app_with_db().await;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/10.1234/test")
                    .header("Accept", "application/vnd.citationstyles.csl+json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let ct = response.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("citationstyles.csl+json"), "unexpected content-type: {ct}");
    }
}
