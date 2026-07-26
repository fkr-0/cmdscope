use crate::{AppConfig, AppModel, HistoryColumn, HistoryEntry, KeyConfig, SearchMode};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{Buffer, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget},
};
use std::borrow::Cow;

/// Render a complete `cmdscope` frame into a Ratatui buffer.
///
/// Rendering is intentionally a pure projection of `AppModel` plus `AppConfig`;
/// keyboard handling and state transitions live elsewhere.
pub fn render(model: &AppModel, config: &AppConfig, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(4),
            Constraint::Length(6),
        ])
        .split(area);

    render_header(model, chunks[0], buf);
    render_history(model, config, chunks[1], buf);
    render_shortcuts(config, chunks[2], buf);
}

fn display_text(input: &str) -> Cow<'_, str> {
    if input.chars().all(|character| !character.is_control()) {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\n' => output.push('⏎'),
            '\r' => output.push('␍'),
            '\t' => output.push('⇥'),
            character if character.is_control() => output.push('�'),
            character => output.push(character),
        }
    }
    Cow::Owned(output)
}

fn render_header(model: &AppModel, area: Rect, buf: &mut Buffer) {
    Paragraph::new(header_lines(model))
        .block(Block::default().borders(Borders::ALL).title("cmdscope"))
        .render(area, buf);
}

fn render_history(model: &AppModel, config: &AppConfig, area: Rect, buf: &mut Buffer) {
    let items = model
        .visible()
        .map(|entry| history_item(entry, model.metadata_visible(), &config.ui.history_columns))
        .collect::<Vec<_>>();
    let title = if model.in_context_mode() {
        format!("context ±{}", model.context_radius())
    } else {
        format!("matches ({})", model.visible_len())
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let selected = (model.visible_len() > 0).then_some(model.selected_index());
    let mut state = ratatui::widgets::ListState::default().with_selected(selected);
    StatefulWidget::render(list, area, buf, &mut state);
}

fn render_shortcuts(config: &AppConfig, area: Rect, buf: &mut Buffer) {
    Paragraph::new(help_lines(config))
        .block(Block::default().borders(Borders::ALL).title("shortcuts"))
        .render(area, buf);
}

fn header_lines<'a>(model: &'a AppModel) -> Vec<Line<'a>> {
    let mode = match model.search_mode() {
        SearchMode::All => Cow::Borrowed("global"),
        SearchMode::SamePwd => Cow::Owned(format!("pwd:{}", model.pwd_match_mode().label())),
        SearchMode::GitRoot => Cow::Borrowed("git-root"),
    };
    vec![
        Line::from(vec![
            Span::styled("mode ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(mode),
            Span::raw("  "),
            Span::styled("query ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(model.query()),
        ]),
        Line::from(if model.in_context_mode() {
            "reviewing time context; filter text is ignored until leaving context".to_string()
        } else {
            "filtering with skim fuzzy matching".to_string()
        }),
    ]
}

fn help_lines(config: &AppConfig) -> Vec<Line<'static>> {
    let selection = format!(
        "{}/{}",
        KeyConfig::display(&config.keys.select_previous),
        KeyConfig::display(&config.keys.select_next)
    );
    vec![
        Line::from(format!(
            "scopes: {} global · {} pwd · {} git-root · {} toggle",
            KeyConfig::display(&config.keys.global),
            KeyConfig::display(&config.keys.pwd),
            KeyConfig::display(&config.keys.git_root),
            KeyConfig::display(&config.keys.toggle_scope),
        )),
        Line::from(format!(
            "pwd/context: {} mode · {} context · {}/{} grow/shrink",
            KeyConfig::display(&config.keys.toggle_pwd_mode),
            KeyConfig::display(&config.keys.context),
            KeyConfig::display(&config.keys.context_expand),
            KeyConfig::display(&config.keys.context_shrink),
        )),
        Line::from(format!(
            "view/nav: {} metadata · {} select",
            KeyConfig::display(&config.keys.toggle_metadata),
            selection,
        )),
        Line::from(format!(
            "finish/edit: {} accept · {} quit · {} backspace",
            KeyConfig::display(&config.keys.accept),
            KeyConfig::display(&config.keys.quit),
            KeyConfig::display(&config.keys.backspace),
        )),
    ]
}

fn history_item<'a>(
    entry: &'a HistoryEntry,
    metadata_visible: bool,
    columns: &[HistoryColumn],
) -> ListItem<'a> {
    let exit = if entry.exit == 0 { "ok " } else { "fail " };
    let mut spans = vec![
        Span::styled(exit, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(display_text(&entry.command)),
    ];
    if metadata_visible {
        for column in columns {
            spans.push(Span::raw("  "));
            spans.push(match column {
                HistoryColumn::Date => Span::raw(format_unix_date(entry.timestamp)),
                HistoryColumn::Pwd => Span::raw(display_text(&entry.cwd)),
            });
        }
    }
    ListItem::new(Line::from(spans))
}

fn format_unix_date(timestamp: i64) -> String {
    let seconds = normalized_timestamp_seconds(timestamp);
    let days = seconds.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

fn normalized_timestamp_seconds(timestamp: i64) -> i64 {
    let magnitude = timestamp.unsigned_abs();
    if magnitude >= 100_000_000_000_000_000 {
        timestamp / 1_000_000_000
    } else if magnitude >= 100_000_000_000_000 {
        timestamp / 1_000_000
    } else if magnitude >= 100_000_000_000 {
        timestamp / 1_000
    } else {
        timestamp
    }
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year, month as u32, day as u32)
}
