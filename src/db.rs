use std::path::PathBuf;

use commonmeta::Data;
use rusqlite::{Connection, OpenFlags, params};

use crate::error::AppError;

#[cfg(test)]
pub(crate) const TEST_DDL: &str = r#"
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

fn from_json<T: serde::de::DeserializeOwned + Default>(s: String) -> T {
    if s.is_empty() {
        T::default()
    } else {
        serde_json::from_str(&s).unwrap_or_default()
    }
}

fn connect(path: &PathBuf) -> Result<Connection, AppError> {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| AppError::Internal(format!("sqlite open '{}': {e}", path.display())))
}

/// Validate that `path` can be opened as a SQLite database with a `works` table.
///
/// Returns `Ok(None)` when the file does not exist or contains no `works` table
/// (e.g. a commonmeta database used only for organisations or settings).
/// Returns `Err` only when the file exists but cannot be opened at all.
pub fn open(path: &std::path::Path) -> Result<Option<PathBuf>, AppError> {
    if !path.exists() {
        tracing::warn!(path = %path.display(), "sqlite file not found, running without local database");
        return Ok(None);
    }
    let path = path.to_path_buf();
    let conn = connect(&path)?;
    let has_works: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='works'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !has_works {
        tracing::info!(path = %path.display(), "sqlite database has no works table, running without local database");
        return Ok(None);
    }
    Ok(Some(path))
}

/// Look up a single DOI in a commonmeta SQLite database.
pub fn lookup(path: &PathBuf, doi: &str) -> Result<Option<Data>, AppError> {
    let id = commonmeta::doi_utils::normalize_doi(doi);
    if id.is_empty() {
        return Ok(None);
    }

    let conn = connect(path)?;

    let result = conn.query_row(SQL, params![id], |row| {
        Ok(Data {
            id: row.get(0)?,
            type_: row.get(1)?,
            url: row.get(2)?,
            title: row.get(3)?,
            additional_titles: from_json(row.get(4)?),
            contributors: from_json(row.get(5)?),
            date_published: row.get(6)?,
            date_updated: row.get(7)?,
            dates: from_json(row.get(8)?),
            publisher: from_json(row.get(9)?),
            container: from_json(row.get(10)?),
            description: row.get(11)?,
            license: from_json(row.get(12)?),
            version: row.get(13)?,
            language: row.get(14)?,
            subjects: from_json(row.get(15)?),
            identifiers: from_json(row.get(16)?),
            relations: from_json(row.get(17)?),
            references: from_json(row.get(18)?),
            funding_references: from_json(row.get(19)?),
            geo_locations: from_json(row.get(20)?),
            files: from_json(row.get(21)?),
            archive_locations: from_json(row.get(22)?),
            provider: row.get(23)?,
            ..Data::default()
        })
    });

    match result {
        Ok(data) => Ok(Some(data)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Internal(format!("sqlite query: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    pub(crate) fn make_test_db(path: &Path) -> PathBuf {
        let conn = Connection::open(path).expect("open test db");
        conn.execute_batch(TEST_DDL).expect("create schema");
        conn.execute(
            r#"INSERT INTO works ("id", "type", "url", "title", "contributors", "date_published", "provider")
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                "https://doi.org/10.1234/test",
                "JournalArticle",
                "https://example.com/test-article",
                "Test Article on Content Negotiation",
                r#"[{"name": "Doe, Jane", "contributorRoles": ["Author"]}]"#,
                "2024-01-15",
                "Crossref"
            ],
        )
        .expect("insert test record");
        path.to_path_buf()
    }

    #[test]
    fn open_returns_none_for_missing_file() {
        let result = open(Path::new("/nonexistent/path/db.sqlite3"));
        assert!(matches!(result, Ok(None)), "expected Ok(None), got {result:?}");
    }

    #[test]
    fn open_returns_none_when_no_works_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no_works.sqlite3");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE organizations (id TEXT PRIMARY KEY, name TEXT);")
            .unwrap();
        drop(conn);
        let result = open(&path);
        assert!(matches!(result, Ok(None)), "expected Ok(None), got {result:?}");
    }

    #[test]
    fn lookup_returns_none_for_unknown_doi() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_db(&dir.path().join("test.sqlite3"));
        let result = lookup(&path, "10.9999/does-not-exist").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn lookup_finds_existing_doi() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_db(&dir.path().join("test.sqlite3"));
        let data = lookup(&path, "10.1234/test").unwrap().expect("should find DOI");
        assert_eq!(data.id, "https://doi.org/10.1234/test");
        assert_eq!(data.title, "Test Article on Content Negotiation");
        assert_eq!(data.url, "https://example.com/test-article");
        assert_eq!(data.type_, "JournalArticle");
    }

    #[test]
    fn lookup_normalises_doi_prefix_form() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_db(&dir.path().join("test.sqlite3"));
        for doi in &[
            "10.1234/test",
            "https://doi.org/10.1234/test",
            "http://dx.doi.org/10.1234/test",
        ] {
            assert!(
                lookup(&path, doi).unwrap().is_some(),
                "should find DOI in form '{doi}'"
            );
        }
    }

    #[test]
    fn lookup_empty_string_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_test_db(&dir.path().join("test.sqlite3"));
        assert!(lookup(&path, "").unwrap().is_none());
    }
}
