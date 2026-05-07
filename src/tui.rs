use crate::{AppConfig, AppModel, HistoryEntry, SearchMode};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{Buffer, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, StatefulWidget, Widget},
};

pub fn render(model: &AppModel, config: &AppConfig, area: Rect, buf: &mut Buffer) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(4),
            Constraint::Length(4),
        ])
        .split(area);

    Paragraph::new(header_lines(model))
        .block(Block::default().borders(Borders::ALL).title("cmdscope"))
        .render(chunks[0], buf);

    let items = model.visible().iter().map(history_item).collect::<Vec<_>>();
    let title = if model.in_context_mode() {
        format!("context ±{}", model.context_radius())
    } else {
        format!("matches ({})", model.visible().len())
    };
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    let mut state =
        ratatui::widgets::ListState::default().with_selected(Some(model.selected_index()));
    StatefulWidget::render(list, chunks[1], buf, &mut state);

    Paragraph::new(help_line(config))
        .block(Block::default().borders(Borders::ALL).title("shortcuts"))
        .render(chunks[2], buf);
}

fn header_lines(model: &AppModel) -> Vec<Line<'static>> {
    let mode = match model.search_mode() {
        SearchMode::All => "global".to_string(),
        SearchMode::SamePwd => format!("pwd:{}", model.pwd_match_mode().label()),
        SearchMode::GitRoot => "git-root".to_string(),
    };
    vec![
        Line::from(vec![
            Span::styled("mode ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(mode),
            Span::raw("  "),
            Span::styled("query ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(model.query().to_string()),
        ]),
        Line::from(if model.in_context_mode() {
            "reviewing time context; filter text is ignored until leaving context".to_string()
        } else {
            "filtering with skim fuzzy matching".to_string()
        }),
    ]
}

fn help_line(config: &AppConfig) -> String {
    format!(
        "{} global · {} pwd · {} git-root · {} pwd-mode · {} context · {}/{} grow/shrink · ↑/↓ select · Enter accept · Esc quit",
        config.keys.global,
        config.keys.pwd,
        config.keys.git_root,
        config.keys.toggle_pwd_mode,
        config.keys.context,
        config.keys.context_expand,
        config.keys.context_shrink,
    )
}

fn history_item(entry: &HistoryEntry) -> ListItem<'static> {
    let exit = if entry.exit == 0 { "ok" } else { "fail" };
    ListItem::new(Line::from(vec![
        Span::styled(
            format!("{} ", exit),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}  ", entry.command)),
        Span::raw(format!("({})", entry.cwd)),
    ]))
}
