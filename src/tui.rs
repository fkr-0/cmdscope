use crate::config::{Alignment as ConfigAlignment, Truncation as ConfigTruncation};
use crate::{AppConfig, AppModel, ColumnId, HistoryEntry, SearchMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::{Buffer, Color, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph, StatefulWidget, Tabs, Widget,
        Wrap,
    },
};
use std::{
    borrow::Cow,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MODE_WIDTH: usize = 14;
const INPUT_PREFIX_WIDTH: usize = MODE_WIDTH + 3;
const MIN_COMMAND_WIDTH: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compactness {
    Ultra,
    Compact,
    Full,
}

fn render_active_window(model: &AppModel, area: Rect, buf: &mut Buffer) {
    let Some(name) = model.active_window() else {
        return;
    };
    let kind = model
        .active_window_config()
        .map(|window| window.kind.as_str())
        .unwrap_or(name);
    let Some(selected) = model.selected() else {
        return;
    };
    let width = model
        .active_window_config()
        .and_then(|window| window.width)
        .unwrap_or(72)
        .min(area.width.saturating_sub(4));
    let height = model
        .active_window_config()
        .and_then(|window| window.height)
        .unwrap_or(14)
        .min(area.height.saturating_sub(2));
    if width < 24 || height < 5 {
        return;
    }
    let popup = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    Clear.render(popup, buf);
    let title = model
        .active_window_config()
        .and_then(|window| window.title.clone())
        .unwrap_or_else(|| format!(" {name} "));
    let mut lines = Vec::new();
    let selected_id = selected.id.as_str();
    for entry in model.inspect_entries(kind) {
        let marker = if entry.id == selected_id { "> " } else { "  " };
        let date = crate::columns::format_column(
            entry,
            ColumnId::Date,
            model.columns(),
            current_unix_seconds(),
        );
        let command = truncate_end_to_width(
            display_text(&entry.command).as_ref(),
            usize::from(width).saturating_sub(15),
        );
        lines.push(Line::from(format!("{marker}{date:>8}  {command}")));
    }
    Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title),
        )
        .wrap(Wrap { trim: false })
        .render(popup, buf);
}

/// Render a complete `cmdscope` frame into a Ratatui buffer.
///
/// The returned position is the real terminal cursor for the query editor. It
/// is absent while reviewing context or when the terminal is too narrow to
/// expose an editable query cell.
pub fn render(
    model: &AppModel,
    config: &AppConfig,
    area: Rect,
    buf: &mut Buffer,
) -> Option<(u16, u16)> {
    if area.is_empty() {
        return None;
    }

    let area = horizontal_inset(area);
    let compactness = compactness(area);
    let constraints = match compactness {
        Compactness::Ultra => vec![Constraint::Min(1), Constraint::Length(1)],
        Compactness::Compact => vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ],
        Compactness::Full => vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(4),
        ],
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    match compactness {
        Compactness::Ultra => {
            render_history(model, config, chunks[0], buf, compactness);
            render_input(model, chunks[1], buf)
        }
        Compactness::Compact => {
            render_header(model, config, chunks[0], buf);
            render_tabs(model, chunks[1], buf);
            render_history(model, config, chunks[2], buf, compactness);
            let cursor = render_input(model, chunks[3], buf);
            render_actions_menu(model, area, buf);
            render_active_window(model, area, buf);
            cursor
        }
        Compactness::Full => {
            render_header(model, config, chunks[0], buf);
            render_tabs(model, chunks[1], buf);
            render_history(model, config, chunks[2], buf, compactness);
            let cursor = render_input(model, chunks[3], buf);
            if config.ui.preview {
                render_preview(model, chunks[4], buf);
            }
            render_actions_menu(model, area, buf);
            render_active_window(model, area, buf);
            cursor
        }
    }
}

fn horizontal_inset(area: Rect) -> Rect {
    if area.width > 2 {
        Rect::new(area.x + 1, area.y, area.width - 2, area.height)
    } else {
        area
    }
}

fn compactness(area: Rect) -> Compactness {
    if area.height <= 4 || area.width < 30 {
        Compactness::Ultra
    } else if area.height < 12 || area.width < 64 {
        Compactness::Compact
    } else {
        Compactness::Full
    }
}

fn display_text(input: &str) -> Cow<'_, str> {
    if input
        .chars()
        .all(|character| !needs_visible_replacement(character))
    {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\n' => output.push('⏎'),
            '\r' => output.push('␍'),
            '\t' => output.push('⇥'),
            character if needs_visible_replacement(character) => output.push('�'),
            character => output.push(character),
        }
    }
    Cow::Owned(output)
}

fn preview_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\n' => output.push('\n'),
            '\r' => output.push('␍'),
            '\t' => output.push('⇥'),
            character if needs_visible_replacement(character) => output.push('�'),
            character => output.push(character),
        }
    }
    output
}

fn needs_visible_replacement(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{061c}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
}

fn render_header(model: &AppModel, config: &AppConfig, area: Rect, buf: &mut Buffer) {
    if area.is_empty() {
        return;
    }

    let title = Line::from(vec![
        Span::styled("cmdscope", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!(" v{VERSION}"), Style::default().fg(Color::DarkGray)),
    ]);
    let count = if model.in_context_mode() {
        format!(
            "{} context · {} total",
            model.visible_len(),
            model.history_count()
        )
    } else {
        format!(
            "{} shown · {} total",
            model.visible_len(),
            model.history_count()
        )
    };
    let count_width = UnicodeWidthStr::width(count.as_str()) as u16;

    if area.width >= 116 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(18),
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(count_width.min(area.width)),
            ])
            .split(area);
        Paragraph::new(title).render(chunks[0], buf);
        Paragraph::new(help_line(model, config))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .render(chunks[1], buf);
        Paragraph::new(count)
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray))
            .render(chunks[3], buf);
    } else if area.width >= 50 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(count_width.min(area.width)),
            ])
            .split(area);
        Paragraph::new(title).render(chunks[0], buf);
        Paragraph::new(count)
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::DarkGray))
            .render(chunks[1], buf);
    } else {
        Paragraph::new(title).render(area, buf);
    }
}

fn render_tabs(model: &AppModel, area: Rect, buf: &mut Buffer) {
    if area.is_empty() {
        return;
    }
    Tabs::new([Line::from("Search"), Line::from("Inspect")])
        .select(usize::from(model.in_context_mode()))
        .divider(" │ ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .render(area, buf);
}

fn render_history(
    model: &AppModel,
    config: &AppConfig,
    area: Rect,
    buf: &mut Buffer,
    compactness: Compactness,
) {
    if area.is_empty() {
        return;
    }

    let title = history_title(model);
    let list_area = if compactness == Compactness::Full {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title);
        let inner = block.inner(area);
        block.render(area, buf);
        inner
    } else {
        area
    };

    if model.visible_len() == 0 {
        let message = if model.query().is_empty() {
            "No history entries in this scope".to_string()
        } else {
            format!(
                "No matches for “{}” · {} clears the query",
                truncate_end_to_width(display_text(model.query()).as_ref(), 28),
                primary_binding(&config.keys.clear_query)
            )
        };
        Paragraph::new(message)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .render(list_area, buf);
        return;
    }

    let content_width = usize::from(list_area.width.saturating_sub(2));
    let now = current_unix_seconds();
    let mut items = model
        .visible()
        .map(|entry| history_item(entry, model, config, now, content_width))
        .collect::<Vec<_>>();
    items.reverse();
    let list = List::new(items).highlight_symbol("> ").highlight_style(
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .add_modifier(Modifier::BOLD),
    );
    let visual_index = model
        .visible_len()
        .saturating_sub(1)
        .saturating_sub(model.selected_index());
    let mut state = ratatui::widgets::ListState::default()
        .with_selected(Some(visual_index))
        .with_offset(visual_index.saturating_sub(2));
    StatefulWidget::render(list, list_area, buf, &mut state);
}

fn history_title(model: &AppModel) -> String {
    if model.in_context_mode() {
        format!(
            " Inspect {}/{} · ±{} ",
            model.selected_index() + 1,
            model.visible_len(),
            model.context_radius()
        )
    } else if model.visible_len() == 0 {
        " 0 matches ".to_string()
    } else {
        format!(
            " {}/{} matches · col {} · sort {}{} ",
            model.selected_index() + 1,
            model.visible_len(),
            model.focused_column(),
            model.sort_field(),
            model.sort_direction().symbol()
        )
    }
}

fn render_input(model: &AppModel, area: Rect, buf: &mut Buffer) -> Option<(u16, u16)> {
    if area.is_empty() {
        return None;
    }

    let mode = input_mode(model);
    let badge = format!("[{mode:^MODE_WIDTH$}] ");
    if usize::from(area.width) <= INPUT_PREFIX_WIDTH {
        Paragraph::new(truncate_end_to_width(&badge, usize::from(area.width)))
            .style(Style::default().fg(Color::Cyan))
            .render(area, buf);
        return None;
    }

    let available = usize::from(area.width) - INPUT_PREFIX_WIDTH;
    let mut spans = vec![Span::styled(
        badge,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];

    if model.in_context_mode() {
        let query = if model.query().is_empty() {
            "query paused".to_string()
        } else {
            let query = truncate_end_to_width(display_text(model.query()).as_ref(), available);
            format!("{query}  (paused)")
        };
        spans.push(Span::styled(
            truncate_end_to_width(&query, available),
            Style::default().fg(Color::DarkGray),
        ));
        Paragraph::new(Line::from(spans)).render(area, buf);
        return None;
    }

    if model.query().is_empty() {
        spans.push(Span::styled(
            truncate_end_to_width("type to filter history", available),
            Style::default().fg(Color::DarkGray),
        ));
        Paragraph::new(Line::from(spans)).render(area, buf);
        return Some((area.x + INPUT_PREFIX_WIDTH as u16, area.y));
    }

    let text_width = available.saturating_sub(1);
    let (window, cursor_column) = query_window(model.query(), model.query_cursor(), text_width);
    spans.push(Span::raw(window));
    Paragraph::new(Line::from(spans)).render(area, buf);

    let cursor_x = area
        .x
        .saturating_add(INPUT_PREFIX_WIDTH as u16)
        .saturating_add(u16::try_from(cursor_column).unwrap_or(u16::MAX))
        .min(area.x.saturating_add(area.width.saturating_sub(1)));
    Some((cursor_x, area.y))
}

fn render_actions_menu(model: &AppModel, area: Rect, buf: &mut Buffer) {
    let Some(menu) = model.actions_menu() else {
        return;
    };
    let width = 30.min(area.width.saturating_sub(4));
    let height = (menu.items.len() as u16 + 2).min(area.height.saturating_sub(2));
    if width < 12 || height < 3 {
        return;
    }
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    Clear.render(popup, buf);
    let items = menu
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let marker = if index == menu.selected { "> " } else { "  " };
            let style = if index == menu.selected {
                Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!("{marker}{}", item.label),
                style,
            )))
        })
        .collect::<Vec<_>>();
    Widget::render(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Actions "),
        ),
        popup,
        buf,
    );
}

fn render_preview(model: &AppModel, area: Rect, buf: &mut Buffer) {
    if area.is_empty() {
        return;
    }

    let (title, text) = model.selected().map_or_else(
        || {
            (
                " selected command ".to_string(),
                "No command selected".to_string(),
            )
        },
        |entry| {
            let mode = if model.in_context_mode() {
                " inspect "
            } else {
                " selected "
            };
            let cwd_budget = usize::from(area.width).saturating_sub(34).min(36);
            let cwd = truncate_start_to_width(display_text(&entry.cwd).as_ref(), cwd_budget);
            (
                format!(
                    "{mode}· exit {} · {} · {cwd} ",
                    entry.exit,
                    format_execution_duration(entry.duration)
                ),
                preview_text(&entry.command),
            )
        },
    );

    Paragraph::new(text)
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(title),
        )
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

fn input_mode(model: &AppModel) -> String {
    if model.in_context_mode() {
        return format!("INSPECT ±{}", model.context_radius());
    }
    match model.search_mode() {
        SearchMode::All => "GLOBAL".to_string(),
        SearchMode::SamePwd => format!("PWD:{}", model.pwd_match_mode().label().to_uppercase()),
        SearchMode::GitRoot => "GIT-ROOT".to_string(),
    }
}

fn help_line(model: &AppModel, config: &AppConfig) -> Line<'static> {
    let entries = if model.in_context_mode() {
        [
            (&config.keys.quit, "exit"),
            (&config.keys.context, "search"),
            (&config.keys.context_expand, "grow"),
            (&config.keys.context_shrink, "shrink"),
        ]
    } else {
        [
            (&config.keys.quit, "exit"),
            (&config.keys.toggle_scope, "scope"),
            (&config.keys.accept, "edit"),
            (&config.keys.context, "inspect"),
        ]
    };
    let mut spans = Vec::new();
    for (index, (bindings, action)) in entries.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(", "));
        }
        spans.push(Span::styled(
            format!("<{}>", primary_binding(bindings)),
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(": {action}")));
    }
    Line::from(spans)
}

fn primary_binding(bindings: &[String]) -> &str {
    bindings.first().map_or("?", String::as_str)
}

fn history_item(
    entry: &HistoryEntry,
    model: &AppModel,
    config: &AppConfig,
    now: i64,
    content_width: usize,
) -> ListItem<'static> {
    let success = entry.exit == 0;
    let status_style = Style::default()
        .fg(if success { Color::Green } else { Color::Red })
        .add_modifier(Modifier::BOLD);
    let mut spans = vec![
        Span::styled(if success { "✓" } else { "×" }, status_style),
        Span::raw(" "),
    ];
    let prefix_width = 2;

    let mut metadata = Vec::<(String, Style)>::new();
    let mut rendered_columns = Vec::new();
    for column in model.columns().visible() {
        if !model.column_visible(column) || (content_width < 60 && column == ColumnId::Pwd) {
            continue;
        }
        let presentation = config.ui.presentation(column);
        let raw = match column {
            ColumnId::Date | ColumnId::Duration => {
                crate::columns::format_column(entry, column, &config.ui.effective_columns(), now)
            }
            ColumnId::Pwd => display_text(&entry.cwd).into_owned(),
            ColumnId::Exit => entry.exit.to_string(),
        };
        let mut width = presentation
            .width
            .unwrap_or_else(|| UnicodeWidthStr::width(raw.as_str()))
            .max(presentation.min_width.unwrap_or(0));
        if let Some(max_width) = presentation.max_width {
            width = width.min(max_width);
        }
        if width == 0 {
            continue;
        }
        let rendered = match presentation.truncation {
            ConfigTruncation::End => crate::columns::truncate_end(&raw, width),
            ConfigTruncation::Start => crate::columns::truncate_start(&raw, width),
            ConfigTruncation::None if UnicodeWidthStr::width(raw.as_str()) <= width => raw,
            ConfigTruncation::None => continue,
        };
        let rendered_width = UnicodeWidthStr::width(rendered.as_str());
        let text = match presentation.align {
            ConfigAlignment::Left => format!("{rendered:<width$}"),
            ConfigAlignment::Right => format!("{rendered:>width$}"),
            ConfigAlignment::Center => {
                let padding = width.saturating_sub(rendered_width);
                format!(
                    "{}{}{}",
                    " ".repeat(padding / 2),
                    rendered,
                    " ".repeat(padding - padding / 2)
                )
            }
        };
        rendered_columns.push((presentation.priority, width, text));
    }
    while rendered_columns
        .iter()
        .map(|(_, width, _)| width + 2)
        .sum::<usize>()
        + prefix_width
        + MIN_COMMAND_WIDTH
        > content_width
    {
        let Some((index, _)) = rendered_columns
            .iter()
            .enumerate()
            .max_by_key(|(index, (priority, _, _))| (*priority, *index))
        else {
            break;
        };
        rendered_columns.remove(index);
    }
    let metadata_width = rendered_columns
        .iter()
        .map(|(_, width, _)| width + 2)
        .sum::<usize>();
    for (_, _, text) in rendered_columns {
        metadata.push((text, Style::default().fg(Color::DarkGray)));
    }

    let command_width = content_width.saturating_sub(prefix_width + metadata_width);
    spans.push(Span::raw(truncate_end_to_width(
        display_text(&entry.command).as_ref(),
        command_width,
    )));
    for (value, style) in metadata {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(value, style));
    }
    ListItem::new(Line::from(spans))
}

fn query_window(query: &str, cursor: usize, max_width: usize) -> (String, usize) {
    if max_width == 0 {
        return (String::new(), 0);
    }
    let left = display_text(&query[..cursor]).into_owned();
    let right = display_text(&query[cursor..]).into_owned();
    let left_width = UnicodeWidthStr::width(left.as_str());
    let right_width = UnicodeWidthStr::width(right.as_str());
    if left_width + right_width <= max_width {
        let mut visible = left;
        visible.push_str(&right);
        return (visible, left_width);
    }
    if right_width == 0 {
        let tail = tail_to_width(&left, max_width.saturating_sub(1)).0;
        let visible = format!("…{tail}");
        return (visible.clone(), UnicodeWidthStr::width(visible.as_str()));
    }
    if left_width == 0 {
        let mut visible = head_to_width(&right, max_width.saturating_sub(1)).0;
        visible.push('…');
        return (visible, 0);
    }

    let desired_left = max_width.saturating_mul(2) / 3;
    let (left_tail, left_clipped) = tail_to_width(&left, desired_left.saturating_sub(1));
    let mut visible_left = String::new();
    if left_clipped {
        visible_left.push('…');
    }
    visible_left.push_str(&left_tail);
    let cursor_column = UnicodeWidthStr::width(visible_left.as_str());
    let remaining = max_width.saturating_sub(cursor_column);
    let (mut visible_right, right_clipped) = head_to_width(&right, remaining);
    if right_clipped && remaining > 0 {
        visible_right = head_to_width(&right, remaining.saturating_sub(1)).0;
        visible_right.push('…');
    }
    visible_left.push_str(&visible_right);
    (visible_left, cursor_column)
}

fn truncate_end_to_width(input: &str, width: usize) -> String {
    if UnicodeWidthStr::width(input) <= width {
        return input.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut output = head_to_width(input, width.saturating_sub(1)).0;
    output.push('…');
    output
}

fn truncate_start_to_width(input: &str, width: usize) -> String {
    if UnicodeWidthStr::width(input) <= width {
        return input.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let tail = tail_to_width(input, width.saturating_sub(1)).0;
    format!("…{tail}")
}

fn head_to_width(input: &str, width: usize) -> (String, bool) {
    let mut output = String::new();
    let mut used = 0;
    let mut clipped = false;
    for character in input.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + character_width > width {
            clipped = true;
            break;
        }
        output.push(character);
        used += character_width;
    }
    clipped |= output.len() < input.len();
    (output, clipped)
}

fn tail_to_width(input: &str, width: usize) -> (String, bool) {
    let mut characters = Vec::new();
    let mut used = 0;
    let mut clipped = false;
    for character in input.chars().rev() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + character_width > width {
            clipped = true;
            break;
        }
        characters.push(character);
        used += character_width;
    }
    clipped |= characters.len() < input.chars().count();
    characters.reverse();
    (characters.into_iter().collect(), clipped)
}

fn current_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or(i64::MAX)
}

fn format_execution_duration(nanoseconds: i64) -> String {
    let nanoseconds = u64::try_from(nanoseconds).unwrap_or(0);
    format_duration(Duration::from_nanos(nanoseconds))
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds > 0 {
        return format_seconds(seconds);
    }
    let nanos = u64::from(duration.subsec_nanos());
    if nanos >= 1_000_000 {
        format!("{}ms", nanos / 1_000_000)
    } else if nanos >= 1_000 {
        format!("{}us", nanos / 1_000)
    } else if nanos > 0 {
        format!("{nanos}ns")
    } else {
        "0s".to_string()
    }
}

fn format_seconds(seconds: u64) -> String {
    const YEAR: u64 = 31_557_600;
    const MONTH: u64 = 2_630_016;
    const DAY: u64 = 86_400;
    const HOUR: u64 = 3_600;
    const MINUTE: u64 = 60;
    for (unit_seconds, suffix) in [
        (YEAR, "y"),
        (MONTH, "mo"),
        (DAY, "d"),
        (HOUR, "h"),
        (MINUTE, "m"),
    ] {
        if seconds >= unit_seconds {
            return format!("{}{suffix}", seconds / unit_seconds);
        }
    }
    format!("{seconds}s")
}

#[allow(dead_code)]
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
