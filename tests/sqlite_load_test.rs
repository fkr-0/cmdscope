use cmdscope::{HistoryStore, SearchMode};
use rusqlite::Connection;
use tempfile::NamedTempFile;

#[test]
fn loads_atuin_history_schema_and_ignores_deleted_rows() {
    let db = NamedTempFile::new().unwrap();
    let connection = Connection::open(db.path()).unwrap();
    connection.execute_batch(
        "create table history (
            id text primary key,
            timestamp integer not null,
            duration integer not null,
            exit integer not null,
            command text not null,
            cwd text not null,
            session text not null,
            hostname text not null,
            deleted_at integer,
            author text,
            intent text,
            unique(timestamp, cwd, command)
        );
        insert into history values ('live', 20, 10, 0, 'cargo build', '/repo', 's1', 'host', null, null, null);
        insert into history values ('deleted', 30, 10, 0, 'rm secret', '/repo', 's1', 'host', 40, null, null);",
    ).unwrap();

    let store = HistoryStore::load_sqlite(db.path()).unwrap();

    assert_eq!(store.len(), 1);
    assert_eq!(
        store.search("", SearchMode::All, None, 10)[0].command,
        "cargo build"
    );
}

#[test]
fn loads_older_schema_without_soft_delete_column() {
    let db = NamedTempFile::new().unwrap();
    let connection = Connection::open(db.path()).unwrap();
    connection
        .execute_batch(
            "create table history (
                id text primary key,
                timestamp integer not null,
                duration integer not null,
                exit integer not null,
                command text not null,
                cwd text not null,
                session text not null,
                hostname text not null
            );
            insert into history values ('old', 20, 10, 0, 'echo old', '/repo', 's1', 'host');",
        )
        .unwrap();

    let store = HistoryStore::load_sqlite(db.path()).unwrap();

    assert_eq!(store.len(), 1);
    assert_eq!(store.entries()[0].command, "echo old");
}

#[test]
fn reports_missing_table_and_required_columns_clearly() {
    let no_table = NamedTempFile::new().unwrap();
    Connection::open(no_table.path()).unwrap();
    let error = HistoryStore::load_sqlite(no_table.path()).unwrap_err();
    let error = format!("{error:#}");
    assert!(
        error.contains("does not contain a history table"),
        "{error}"
    );
    assert!(
        error.contains(&no_table.path().display().to_string()),
        "{error}"
    );

    let incomplete = NamedTempFile::new().unwrap();
    let connection = Connection::open(incomplete.path()).unwrap();
    connection
        .execute_batch("create table history (id text primary key, timestamp integer not null);")
        .unwrap();
    let error = HistoryStore::load_sqlite(incomplete.path()).unwrap_err();
    let error = format!("{error:#}");
    assert!(error.contains("missing required columns"), "{error}");
    assert!(error.contains("duration"), "{error}");
    assert!(error.contains("hostname"), "{error}");
}

#[test]
fn unordered_sqlite_rows_are_sorted_chronologically_in_memory() {
    let db = NamedTempFile::new().unwrap();
    let connection = Connection::open(db.path()).unwrap();
    connection
        .execute_batch(
            "create table history (
                id text primary key,
                timestamp integer not null,
                duration integer not null,
                exit integer not null,
                command text not null,
                cwd text not null,
                session text not null,
                hostname text not null
            );
            insert into history values ('late', 30, 0, 0, 'late command', '/repo', 's', 'h');
            insert into history values ('early', 10, 0, 0, 'early command', '/repo', 's', 'h');
            insert into history values ('middle', 20, 0, 0, 'middle command', '/repo', 's', 'h');",
        )
        .unwrap();

    let store = HistoryStore::load_sqlite(db.path()).unwrap();
    let ids = store
        .entries()
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["early", "middle", "late"]);
}

#[test]
fn equal_timestamps_have_deterministic_id_order() {
    let db = NamedTempFile::new().unwrap();
    let connection = Connection::open(db.path()).unwrap();
    connection
        .execute_batch(
            "create table history (
                id text primary key,
                timestamp integer not null,
                duration integer not null,
                exit integer not null,
                command text not null,
                cwd text not null,
                session text not null,
                hostname text not null
            );
            insert into history values ('z-id', 20, 0, 0, 'z command', '/repo', 's', 'h');
            insert into history values ('a-id', 20, 0, 0, 'a command', '/repo', 's', 'h');",
        )
        .unwrap();

    let store = HistoryStore::load_sqlite(db.path()).unwrap();
    let ids = store
        .search("", SearchMode::All, None, 10)
        .into_iter()
        .map(|entry| entry.id)
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["z-id", "a-id"]);
}

#[test]
fn incompatible_row_values_include_database_context() {
    let db = NamedTempFile::new().unwrap();
    let connection = Connection::open(db.path()).unwrap();
    connection
        .execute_batch(
            "create table history (
                id text primary key,
                timestamp integer not null,
                duration integer not null,
                exit integer not null,
                command blob not null,
                cwd text not null,
                session text not null,
                hostname text not null
            );
            insert into history values ('bad', 20, 0, 0, x'ff', '/repo', 's', 'h');",
        )
        .unwrap();

    let error = HistoryStore::load_sqlite(db.path()).unwrap_err();
    let error = format!("{error:#}");

    assert!(error.contains("incompatible values"), "{error}");
    assert!(error.contains(&db.path().display().to_string()), "{error}");
}
