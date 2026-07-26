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

#[test]
fn footer_shows_all_configurable_action_groups() {
    let config = AppConfig::from_toml(
        r#"
        [keys]
        toggle_scope = "ctrl-t"
        backspace = "delete"
        "#,
    )
    .unwrap();

    let text = rendered_text(&model(), &config);

    assert!(text.contains("ctrl-t toggle"), "{text}");
    assert!(text.contains("delete backspace"), "{text}");
    assert!(text.contains("finish/edit:"), "{text}");
}

#[test]
fn empty_results_render_without_an_invalid_selection() {
    let mut model = model();
    for character in "no-such-command".chars() {
        model.update(Msg::Input(character));
    }

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("matches (0)"), "{text}");
    assert!(!text.contains('▶'), "{text}");
}

#[test]
fn future_second_timestamps_are_not_misclassified_as_milliseconds() {
    let model = AppModel::new(
        HistoryStore::from_entries(vec![HistoryEntry::new(
            "1",
            10_413_792_000,
            0,
            "future",
            "/repo",
            "s",
            "h",
        )]),
        Some("/repo".to_string()),
    );

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("2300-01-01"), "{text}");
}

#[test]
fn common_timestamp_precisions_render_the_same_date() {
    for timestamp in [
        1_700_000_000_i64,
        1_700_000_000_000,
        1_700_000_000_000_000,
        1_700_000_000_000_000_000,
    ] {
        let model = AppModel::new(
            HistoryStore::from_entries(vec![HistoryEntry::new(
                timestamp.to_string(),
                timestamp,
                0,
                "timestamp",
                "/repo",
                "s",
                "h",
            )]),
            Some("/repo".to_string()),
        );
        let text = rendered_text(&model, &AppConfig::default());
        assert!(text.contains("2023-11-14"), "timestamp={timestamp}\n{text}");
    }
}

#[test]
fn control_characters_are_rendered_as_safe_single_line_symbols() {
    let model = AppModel::new(
        HistoryStore::from_entries(vec![HistoryEntry::new(
            "1",
            1_700_000_000,
            0,
            "printf one\ntwo\rthree\tfour\u{1b}[31m",
            "/repo\nunsafe",
            "s",
            "h",
        )]),
        Some("/repo".to_string()),
    );

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("one⏎two␍three⇥four�[31m"), "{text}");
    assert!(text.contains("/repo⏎unsafe"), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");
    assert!(!text.contains('\t'), "{text}");
}

fn rendered_text(model: &AppModel, config: &AppConfig) -> String {
    let area = Rect::new(0, 0, 100, 14);
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
