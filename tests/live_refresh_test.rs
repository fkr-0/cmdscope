use cmdscope::live::LiveHistory;
use cmdscope::{AppModel, HistoryStore};
use rusqlite::Connection;

fn create_db(path: &std::path::Path) {
    let db = Connection::open(path).unwrap();
    db.execute_batch("create table history (id text primary key, timestamp integer not null, duration integer not null, exit integer not null, command text not null, cwd text not null, session text not null, hostname text not null, deleted_at integer);").unwrap();
}
fn insert(path: &std::path::Path, id: &str, timestamp: i64, command: &str) {
    let db = Connection::open(path).unwrap();
    db.execute(
        "insert into history values (?1,?2,0,0,?3,'/repo','s','h',null)",
        (id, timestamp, command),
    )
    .unwrap();
}

#[test]
fn sqlite_refresh_insert_update_delete_and_selection_disappearance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.db");
    create_db(&path);
    insert(&path, "a", 100, "alpha");
    insert(&path, "b", 200, "beta");
    let initial = HistoryStore::load_sqlite(&path).unwrap();
    let mut live = LiveHistory::new(&path).unwrap();
    assert!(live.load_if_changed().unwrap().is_none());
    insert(&path, "c", 300, "gamma");
    let added = live.load_if_changed().unwrap().unwrap();
    assert_eq!(
        added
            .entries()
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
    let db = Connection::open(&path).unwrap();
    db.execute("update history set command='beta-updated' where id='b'", [])
        .unwrap();
    let updated = live.load_if_changed().unwrap().unwrap();
    assert_eq!(
        updated
            .entries()
            .iter()
            .find(|e| e.id == "b")
            .unwrap()
            .command,
        "beta-updated"
    );
    db.execute("update history set deleted_at=1 where id='b'", [])
        .unwrap();
    let deleted = live.load_if_changed().unwrap().unwrap();
    assert!(deleted.entries().iter().all(|e| e.id != "b"));
    let mut model = AppModel::new(initial, Some("/repo".into()));
    assert_eq!(model.selected_id(), Some("b"));
    model.replace_history(updated);
    assert_eq!(model.selected_id(), Some("b"));
    model.replace_history(deleted);
    assert_eq!(model.selected_id(), Some("a"));
}
