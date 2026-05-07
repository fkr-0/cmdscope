use rusqlite::Connection;
use tempfile::NamedTempFile;
use cmdscope::{HistoryStore, SearchMode};

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
