use commonmeta::Data;

use crate::error::AppError;

/// SQLite DDL mirroring the commonmeta `works` table schema.
#[cfg(test)]
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

const SQL: &str = r#"SELECT
    "id", "type", "url", "title", "additional_titles",
    "contributors", "date_published", "date_updated", "dates", "publisher",
    "container", "description", "license",
    "version", "language", "subjects", "identifiers", "relations", "references",
    "funding_references", "geo_locations", "files", "archive_locations", "provider"
FROM works WHERE id = ?1"#;

/// Open a commonmeta SQLite database at `path`.
///
/// `libsql::Builder::new_local(...).build()` only records the path; the file
/// is not actually opened until `connect()` is called.  This function probes
/// with a real connection so that a missing or unreadable file is caught at
/// startup rather than on the first request.
///
/// The returned `Database` should be stored in application state and reused
/// across requests.
pub async fn open(path: &std::path::Path) -> Result<libsql::Database, AppError> {
    if !path.exists() {
        return Err(AppError::Internal(format!(
            "sqlite file not found: '{}'",
            path.display()
        )));
    }

    let db = libsql::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| AppError::Internal(format!("sqlite build '{}': {e}", path.display())))?;

    // Verify the connection actually works before accepting traffic.
    db.connect()
        .map_err(|e| AppError::Internal(format!("sqlite connect '{}': {e}", path.display())))?;

    Ok(db)
}

/// Look up a single DOI in a commonmeta SQLite database.
///
/// The `doi` argument may be in any form accepted by `validate_doi` (bare
/// `10.xxx/yyy`, `https://doi.org/...`, etc.).  It is normalised to the
/// canonical `https://doi.org/10.xxx/yyy` form used as the primary key in
/// the `works` table.
///
/// Returns `Ok(Some(data))` on a hit, `Ok(None)` when the DOI is absent,
/// or `Err` on an I/O or query error.
pub async fn lookup(db: &libsql::Database, doi: &str) -> Result<Option<Data>, AppError> {
    let id = commonmeta::doi_utils::normalize_doi(doi);
    if id.is_empty() {
        return Ok(None);
    }

    let conn = db
        .connect()
        .map_err(|e| AppError::Internal(format!("sqlite connect: {e}")))?;

    let mut rows = conn
        .query(SQL, libsql::params![id])
        .await
        .map_err(|e| AppError::Internal(format!("sqlite query: {e}")))?;

    match rows
        .next()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
    {
        None => Ok(None),
        Some(row) => {
            fn j<T: serde::de::DeserializeOwned + Default>(s: String) -> T {
                if s.is_empty() {
                    T::default()
                } else {
                    serde_json::from_str(&s).unwrap_or_default()
                }
            }
            macro_rules! s {
                ($i:literal) => {
                    row.get::<String>($i).unwrap_or_default()
                };
            }
            Ok(Some(Data {
                id: s!(0),
                type_: s!(1),
                url: s!(2),
                title: s!(3),
                additional_titles: j(s!(4)),
                contributors: j(s!(5)),
                date_published: s!(6),
                date_updated: s!(7),
                dates: j(s!(8)),
                publisher: j(s!(9)),
                container: j(s!(10)),
                description: s!(11),
                license: j(s!(12)),
                version: s!(13),
                language: s!(14),
                subjects: j(s!(15)),
                identifiers: j(s!(16)),
                relations: j(s!(17)),
                references: j(s!(18)),
                funding_references: j(s!(19)),
                geo_locations: j(s!(20)),
                files: j(s!(21)),
                archive_locations: j(s!(22)),
                provider: s!(23),
                ..Data::default()
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Create a minimal test database at `path` with one record.
    async fn make_test_db(path: &Path) -> libsql::Database {
        let db = libsql::Builder::new_local(path)
            .build()
            .await
            .expect("build test db");
        let conn = db.connect().expect("connect test db");
        conn.execute_batch(TEST_DDL).await.expect("create schema");
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
        .expect("insert test record");
        db
    }

    #[tokio::test]
    async fn open_rejects_missing_file() {
        let result = open(Path::new("/nonexistent/path/db.sqlite3")).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not found"), "unexpected error: {msg}");
    }

    #[tokio::test]
    async fn lookup_returns_none_for_unknown_doi() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let db = make_test_db(&path).await;

        let result = lookup(&db, "10.9999/does-not-exist").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn lookup_finds_existing_doi() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let db = make_test_db(&path).await;

        let data = lookup(&db, "10.1234/test").await.unwrap().expect("should find DOI");
        assert_eq!(data.id, "https://doi.org/10.1234/test");
        assert_eq!(data.title, "Test Article on Content Negotiation");
        assert_eq!(data.url, "https://example.com/test-article");
        assert_eq!(data.type_, "JournalArticle");
    }

    #[tokio::test]
    async fn lookup_normalises_doi_prefix_form() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let db = make_test_db(&path).await;

        // Bare DOI, HTTPS URL form, and HTTP URL form should all resolve to the same record.
        for doi in &[
            "10.1234/test",
            "https://doi.org/10.1234/test",
            "http://dx.doi.org/10.1234/test",
        ] {
            assert!(
                lookup(&db, doi).await.unwrap().is_some(),
                "should find DOI in form '{doi}'"
            );
        }
    }

    #[tokio::test]
    async fn lookup_empty_string_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.sqlite3");
        let db = make_test_db(&path).await;

        let result = lookup(&db, "").await.unwrap();
        assert!(result.is_none());
    }
}
