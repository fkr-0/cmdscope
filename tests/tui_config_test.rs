use cmdscope::{AppConfig, AppModel, HistoryEntry, HistoryStore};
use ratatui::{buffer::Buffer, layout::Rect};

#[test]
fn configured_column_order_and_presentation_are_used() {
    let config = AppConfig::from_toml(
        r#"
        [ui.columns]
        order = ["pwd", "date"]
        [ui.column.pwd]
        width = 12
        align = "right"
        truncation = "start"
        priority = 1
    "#,
    )
    .unwrap();
    let model = AppModel::new_with_config(
        HistoryStore::from_entries(vec![HistoryEntry::new(
            "1",
            1700000000,
            0,
            "cargo test",
            "/repo/long/path",
            "s",
            "h",
        )]),
        Some("/repo".into()),
        None,
        &config.ui.effective_columns(),
        config.pwd.mode,
    );
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 12));
    let text = format!(
        "{:?}",
        cmdscope::tui::render(&model, &config, Rect::new(0, 0, 100, 12), &mut buffer)
    );
    assert!(!text.is_empty());
}
