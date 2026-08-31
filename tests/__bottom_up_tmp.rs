use cmdscope::{AppConfig, AppModel, HistoryEntry, HistoryStore, Msg, tui};
use ratatui::{buffer::Buffer, layout::Rect};
#[test]
fn order_and_selection() {
    let mut m = AppModel::new(
        HistoryStore::from_entries(vec![
            HistoryEntry::new("old", 100, 0, "old command", "/repo", "s", "h"),
            HistoryEntry::new("mid", 200, 0, "mid command", "/repo", "s", "h"),
            HistoryEntry::new("new", 300, 0, "new command", "/repo", "s", "h"),
        ]),
        Some("/repo".into()),
    );
    assert_eq!(m.selected_id(), Some("new"));
    m.update(Msg::SelectNext);
    assert_eq!(m.selected_id(), Some("mid"));
    let area = Rect::new(0, 0, 100, 14);
    let mut b = Buffer::empty(area);
    tui::render(&m, &AppConfig::default(), area, &mut b);
    let t = (0..14)
        .map(|y| {
            (0..100)
                .map(|x| b.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let o = t.find("old command").unwrap();
    let mm = t.find("mid command").unwrap();
    let n = t.find("new command").unwrap();
    let q = t.find("type to filter history").unwrap();
    assert!(o < mm && mm < n && n < q, "{t}");
}
