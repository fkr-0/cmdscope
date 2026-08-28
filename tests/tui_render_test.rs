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
fn query_uses_a_real_cursor_and_keeps_it_visible_while_scrolling() {
    let mut model = model();
    for character in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ".chars() {
        model.update(Msg::Input(character));
    }

    let (text_at_end, cursor_at_end) = rendered_text_at(&model, &AppConfig::default(), 70, 14);
    let cursor_at_end = cursor_at_end.expect("search mode exposes a cursor");
    assert!(text_at_end.contains('…'), "{text_at_end}");
    assert!(cursor_at_end.0 < 70, "cursor={cursor_at_end:?}");

    model.update(Msg::CursorStart);
    let (_, cursor_at_start) = rendered_text_at(&model, &AppConfig::default(), 70, 14);
    assert!(cursor_at_start.expect("cursor").0 < cursor_at_end.0);
}

#[test]
fn narrow_rows_prioritize_status_and_command_over_optional_metadata() {
    let (text, _) = rendered_text_at(&model(), &AppConfig::default(), 46, 10);

    assert!(text.contains('✓'), "{text}");
    assert!(text.contains("cargo test"), "{text}");
    assert!(!text.contains("2023-11-14"), "{text}");
    assert!(!text.contains("/repo/src"), "{text}");
}

#[test]
fn failed_commands_have_a_non_color_status_indicator() {
    let model = AppModel::new(
        HistoryStore::from_entries(vec![HistoryEntry::new(
            "1",
            1_700_000_000,
            17,
            "false",
            "/repo",
            "s",
            "h",
        )]),
        Some("/repo".to_string()),
    );

    let text = rendered_text(&model, &AppConfig::default());
    assert!(text.contains('×'), "{text}");
}

#[test]
fn preview_preserves_multiline_structure_without_terminal_controls() {
    let model = AppModel::new(
        HistoryStore::from_entries(vec![HistoryEntry::new(
            "1",
            1_700_000_000,
            0,
            "printf one\ntwo\u{1b}[31m",
            "/repo",
            "s",
            "h",
        )]),
        Some("/repo".to_string()),
    );

    let text = rendered_text(&model, &AppConfig::default());
    assert!(text.contains("printf one"), "{text}");
    assert!(text.contains("two�[31m"), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");
}

#[test]
fn atuin_inspired_chrome_shows_title_tabs_count_and_scope_input() {
    let text = rendered_text(&model(), &AppConfig::default());

    assert!(text.contains("cmdscope v"), "rendered output:\n{text}");
    assert!(text.contains("Search"), "rendered output:\n{text}");
    assert!(text.contains("Inspect"), "rendered output:\n{text}");
    assert!(
        text.contains("2 shown · 2 total"),
        "rendered output:\n{text}"
    );
    assert!(
        text.contains("[    GLOBAL    ]"),
        "rendered output:\n{text}"
    );
}

#[test]
fn context_mode_activates_inspect_tab_and_input_badge() {
    let mut model = model();
    model.update(Msg::ToggleContext);

    let text = rendered_text(&model, &AppConfig::default());

    assert!(
        text.contains("Inspect 2/2 · ±1"),
        "rendered output:\n{text}"
    );
    assert!(
        text.contains("[  INSPECT ±1  ]"),
        "rendered output:\n{text}"
    );
}

#[test]
fn query_bidi_controls_are_rendered_visibly() {
    let mut model = model();
    for character in "git\u{202e}status".chars() {
        model.update(Msg::Input(character));
    }

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("git�status"), "{text}");
    assert!(!text.contains('\u{202e}'), "{text}");
}

#[test]
fn bidi_and_zero_width_controls_are_rendered_visibly() {
    let model = AppModel::new(
        HistoryStore::from_entries(vec![HistoryEntry::new(
            "1",
            1_700_000_000,
            0,
            "safe\u{202e}txt\u{2066}end\u{200b}",
            "/repo\u{200f}path",
            "s",
            "h",
        )]),
        Some("/repo".to_string()),
    );

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("safe�txt�end�"), "{text}");
    assert!(text.contains("/repo�path"), "{text}");
    assert!(!text.contains('\u{202e}'), "{text}");
    assert!(!text.contains('\u{2066}'), "{text}");
}

#[test]
fn rendering_is_safe_for_tiny_terminal_areas() {
    let model = model();
    let config = AppConfig::default();

    for width in 0..=16 {
        for height in 0..=16 {
            let area = Rect::new(0, 0, width, height);
            let mut buffer = Buffer::empty(area);
            tui::render(&model, &config, area, &mut buffer);
        }
    }
}

#[test]
fn header_uses_configured_primary_action_hints() {
    let config = AppConfig::from_toml(
        r#"
        [keys]
        toggle_scope = "ctrl-t"
        context = "ctrl-i"
        "#,
    )
    .unwrap();

    let text = rendered_text_at(&model(), &config, 140, 14).0;

    assert!(text.contains("<ctrl-t>: scope"), "{text}");
    assert!(text.contains("<ctrl-i>: inspect"), "{text}");
    assert!(text.contains("<enter>: edit"), "{text}");
}

#[test]
fn empty_results_render_without_an_invalid_selection() {
    let mut model = model();
    for character in "no-such-command".chars() {
        model.update(Msg::Input(character));
    }

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("0 matches"), "{text}");
    assert!(text.contains("No matches for"), "{text}");
    assert!(text.contains("ctrl-u clears"), "{text}");
    assert!(!text.contains("> "), "{text}");
}

#[test]
fn context_view_renders_neighbor_dates_in_chronological_order_across_sessions() {
    let mut model = AppModel::new(
        HistoryStore::from_entries(vec![
            HistoryEntry::new("1", 1_700_000_000, 0, "first", "/repo", "s1", "h"),
            HistoryEntry::new("2", 1_700_086_400, 0, "second", "/repo", "s2", "h"),
            HistoryEntry::new("3", 1_700_172_800, 0, "third", "/repo", "s3", "h"),
        ]),
        Some("/repo".to_string()),
    );
    model.update(Msg::SelectNext);
    model.update(Msg::ToggleContext);

    let text = rendered_text_at(&model, &AppConfig::default(), 120, 14).0;
    let first = text.find("2023-11-14").expect("first date");
    let second = text.find("2023-11-15").expect("second date");
    let third = text.find("2023-11-16").expect("third date");

    assert!(first < second && second < third, "{text}");
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
    rendered_text_at(model, config, 100, 14).0
}

fn rendered_text_at(
    model: &AppModel,
    config: &AppConfig,
    width: u16,
    height: u16,
) -> (String, Option<(u16, u16)>) {
    let area = Rect::new(0, 0, width, height);
    let mut buffer = Buffer::empty(area);
    let cursor = tui::render(model, config, area, &mut buffer);

    let text = (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buffer.cell((x, y)).expect("cell in bounds").symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    (text, cursor)
}

#[test]
fn selected_row_has_visible_marker() {
    let mut model = model();
    model.update(Msg::SelectNext);

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("> "), "rendered output:\n{text}");
    assert!(text.contains("git status"), "rendered output:\n{text}");
}

#[test]
fn default_render_includes_status_date_and_pwd_metadata() {
    let text = rendered_text(&model(), &AppConfig::default());

    assert!(text.contains("0s"), "rendered output:\n{text}");
    assert!(text.contains("2023-11-14"), "rendered output:\n{text}");
    assert!(text.contains("/repo"), "rendered output:\n{text}");
}

#[test]
fn metadata_can_be_toggled_off_at_runtime() {
    let mut model = model();
    model.update(Msg::ToggleMetadata);

    let text = rendered_text(&model, &AppConfig::default());

    assert!(text.contains("git status"), "rendered output:\n{text}");
    assert!(
        !text.contains("git status  2023-11-14"),
        "rendered output:\n{text}"
    );
    assert!(
        !text.contains("git status  /repo"),
        "rendered output:\n{text}"
    );
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
