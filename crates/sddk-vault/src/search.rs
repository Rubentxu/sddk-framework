//! Rebuildable SQLite FTS5 search index over vault content.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use thiserror::Error;

use crate::index::VaultIndex;

/// One full-text search hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SearchHit {
    /// Matched node id.
    pub id: String,
    /// Node kind.
    pub kind: String,
    /// Node title.
    pub title: String,
    /// Node path.
    pub path: String,
}

/// Errors emitted by the search index.
#[derive(Debug, Error)]
pub enum SearchIndexError {
    /// SQLite rejected an index operation.
    #[error("vault search index error: {0}")]
    Database(#[from] rusqlite::Error),
    /// A filesystem operation failed.
    #[error("vault search index I/O failure: {0}")]
    Io(#[from] std::io::Error),
}

const FTS_TABLE: &str = "vault_fts";

/// Destroys and rebuilds the FTS index from a parsed vault.
///
/// The index is fully derivable from the vault, so rebuilding is the canonical
/// recovery path: drop, re-create, re-insert.
pub fn rebuild_search_index(
    connection: &Connection,
    index: &VaultIndex,
) -> Result<(), SearchIndexError> {
    connection.execute_batch(&format!(
        "DROP TABLE IF EXISTS {FTS_TABLE};
         CREATE VIRTUAL TABLE {FTS_TABLE} USING fts5(id, kind, title, path, body);"
    ))?;
    let mut statement = connection.prepare(&format!(
        "INSERT INTO {FTS_TABLE} (id, kind, title, path, body) VALUES (?1, ?2, ?3, ?4, ?5)"
    ))?;
    for node in &index.nodes {
        statement.execute(params![
            node.id,
            serde_json::to_string(&node.kind).unwrap_or_default(),
            node.title,
            node.path,
            node.body,
        ])?;
    }
    Ok(())
}

/// Searches the FTS index with a sanitized query.
///
/// The query is wrapped in quotes so FTS5 operators in user input are treated
/// as literal text.
pub fn search_index(
    connection: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, SearchIndexError> {
    let sanitized = query.replace('"', "\"\"");
    let mut statement = connection.prepare(&format!(
        "SELECT id, kind, title, path FROM {FTS_TABLE}
         WHERE {FTS_TABLE} MATCH ?1 ORDER BY rank LIMIT ?2"
    ))?;
    let rows = statement.query_map(params![format!("\"{sanitized}\""), limit as i64], |row| {
        Ok(SearchHit {
            id: row.get(0)?,
            kind: row.get(1)?,
            title: row.get(2)?,
            path: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Opens (or creates) the index database at a path.
pub fn open_index(path: &Path) -> Result<Connection, SearchIndexError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(Connection::open(path)?)
}

/// Reports whether a table exists (used by tests to verify rebuildability).
pub fn index_has_rows(connection: &Connection) -> Result<bool, SearchIndexError> {
    let count: Option<i64> = connection
        .query_row(&format!("SELECT COUNT(*) FROM {FTS_TABLE}"), [], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(count.unwrap_or(0) > 0)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;

    use crate::parser::parse_vault;

    use super::{index_has_rows, open_index, rebuild_search_index, search_index};

    fn node(file: &str, content: &str) {
        fs::create_dir_all(std::path::Path::new(file).parent().unwrap()).unwrap();
        fs::write(file, content).unwrap();
    }

    #[test]
    fn index_is_rebuildable_after_deletion() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\n---\n# Auth\n\nOAuth token exchange\n",
        );
        let db_path = directory.path().join("index.sqlite");
        let index = parse_vault(directory.path()).unwrap();

        let connection = open_index(&db_path).unwrap();
        rebuild_search_index(&connection, &index).unwrap();
        let hits = search_index(&connection, "token", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "TERM-A");

        connection.execute_batch("DROP TABLE vault_fts").unwrap();

        rebuild_search_index(&connection, &index).unwrap();
        assert!(index_has_rows(&connection).unwrap());
        assert_eq!(search_index(&connection, "token", 10).unwrap().len(), 1);
    }

    #[test]
    fn query_operators_are_treated_as_literals() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\n---\n# Auth\n\nOAuth token\n",
        );
        let connection = Connection::open_in_memory().unwrap();
        let index = parse_vault(directory.path()).unwrap();
        rebuild_search_index(&connection, &index).unwrap();
        let hits = search_index(&connection, "NEAR(token OR auth", 10).unwrap();
        assert!(hits.is_empty());
    }
}
