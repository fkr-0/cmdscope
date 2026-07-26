use cmdscope::{AppConfig, AppModel, HistoryEntry, HistoryStore, Msg, tui};
use ratatui::{buffer::Buffer, layout::Rect};

fn model() -> AppModel {
    AppModel::new(
        HistoryStore::from_entries(vec![
            HistoryEntry::new("1", 1_700_000_000, 0, "git status", "/repo", "s1", "host"),
            HistoryEntry::new(
                "2",
                1_700_000_060,
                0,
                "cargo test",
                "/repo/src",
                "s1",
                "host",
            ),
        ]),
        Some("/repo".to_string()),
    )
}

fn rendered_text(model: &AppModel, config: &AppConfig) -> String {
    let area = Rect::new(0, 0, 100, 12);
    let mut buffer = Buffer::empty(area);
    tui::render(model, config, area, &mut buffer);

    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buffer.cell((x, y)).expect("cell in bounds").symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn selected_row_has_visible_marker() {
    let mut model = model();
    model.update(Msg::SelectNext);

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("▶ ok git status"), "rendered output:\n{text}");
}

#[test]
fn default_render_includes_status_date_and_pwd_metadata() {
    let text = rendered_text(&model(), &AppConfig::default());

    assert!(text.contains("ok"), "rendered output:\n{text}");
    assert!(text.contains("2023-11-14"), "rendered output:\n{text}");
    assert!(text.contains("/repo"), "rendered output:\n{text}");
}

#[test]
fn metadata_can_be_toggled_off_at_runtime() {
    let mut model = model();
    model.update(Msg::ToggleMetadata);

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("git status"), "rendered output:\n{text}");
    assert!(!text.contains("2023-11-14"), "rendered output:\n{text}");
    assert!(!text.contains("/repo"), "rendered output:\n{text}");
}

#[test]
fn config_can_limit_history_metadata_columns() {
    let config = AppConfig::from_toml(
        r#"
        [ui]
        history_columns = ["pwd"]
        "#,
    )
    .unwrap();

    let text = rendered_text(&model(), &config);

    assert!(!text.contains("2023-11-14"), "rendered output:\n{text}");
    assert!(text.contains("/repo"), "rendered output:\n{text}");
}
