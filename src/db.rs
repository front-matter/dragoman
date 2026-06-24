use commonmeta::Data;

use crate::error::AppError;

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
