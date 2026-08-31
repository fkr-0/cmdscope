use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRegions {
    pub history: Rect,
    pub query: Rect,
    pub preview: Option<Rect>,
    pub header: Option<Rect>,
}

pub fn regions(area: Rect, preview: bool) -> LayoutRegions {
    if area.is_empty() {
        return LayoutRegions {
            history: area,
            query: area,
            preview: None,
            header: None,
        };
    }
    let compact = area.height <= 4 || area.width < 30;
    if compact {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        return LayoutRegions {
            history: chunks[0],
            query: chunks[1],
            preview: None,
            header: None,
        };
    }
    let show_preview = preview && area.height >= 12 && area.width >= 64;
    let constraints = if show_preview {
        vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(5),
        ]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    LayoutRegions {
        header: Some(chunks[0]),
        history: chunks[1],
        query: chunks[2],
        preview: show_preview.then_some(chunks[3]),
    }
}
