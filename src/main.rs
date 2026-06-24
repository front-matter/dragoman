use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use clap::{Args, Parser, Subcommand};
use serde::Deserialize;
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
    #[arg(short, long, env = "PORT", default_value_t = 3000)]
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

    let app = Router::new()
        .route("/", get(index))
        .route("/{*path}", get(handle_pid))
        .with_state(AppState {
            db: database,
        });

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

async fn index() -> &'static str {
    concat!(
        "dragoman — PID redirection and content negotiation server\n\n",
        "Usage: GET /{doi}  (e.g. /10.5281/zenodo.1234)\n\n",
        "Content negotiation via Accept header or ?format= query parameter.\n",
        "Supported formats: commonmeta, csl, datacite, datacite_xml, crossref_xml,\n",
        "                   bibtex, ris, schemaorg, citation\n\n",
        "For formatted citations:\n",
        "  Accept: text/x-bibliography; style=apa; locale=fr-FR\n",
        "  or: ?format=citation&style=apa&locale=fr-FR\n\n",
        "HTTP 302  text/html or */* → redirect to landing page\n",
        "HTTP 406  unsupported content type\n",
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
