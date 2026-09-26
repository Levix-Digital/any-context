use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, AppStatus, MessageRole};

pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();

    // Check if thinking accordion should be displayed
    let show_accordion = app.accordion_open;

    // Vertical Layout
    let chunks = if show_accordion {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(6),    // Chat viewport
                Constraint::Length(7), // Accordion (thinking & tool logs)
                Constraint::Length(3), // Input area
                Constraint::Length(1), // Footer
            ])
            .split(size)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(8),    // Chat viewport
                Constraint::Length(3), // Input area
                Constraint::Length(1), // Footer
            ])
            .split(size)
    };

    render_header(frame, chunks[0], app);
    render_chat(frame, chunks[1], app);

    if show_accordion {
        render_accordion(frame, chunks[2], app);
        render_input(frame, chunks[3], app);
        render_footer(frame, chunks[4], app);
        if app.slash_palette_open {
            render_slash_palette(frame, chunks[3], app);
        }
    } else {
        render_input(frame, chunks[2], app);
        render_footer(frame, chunks[3], app);
        if app.slash_palette_open {
            render_slash_palette(frame, chunks[2], app);
        }
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let status_span = match &app.status {
        AppStatus::Idle => Span::styled("● IDLE", Style::default().fg(Color::Green)),
        AppStatus::Thinking => Span::styled("◐ THINKING", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        AppStatus::Streaming => Span::styled("◕ STREAMING", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        AppStatus::Error(e) => Span::styled(format!("✖ ERROR ({})", e), Style::default().fg(Color::Red)),
    };

    let title = Line::from(vec![
        Span::styled("AnyContext ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(format!("v{} ", env!("CARGO_PKG_VERSION")), Style::default().fg(Color::DarkGray)),
        Span::raw("─ [Workspace: "),
        Span::styled(&app.active_workspace, Style::default().fg(Color::Yellow)),
        Span::raw("] ─ [Model: "),
        Span::styled(&app.active_model, Style::default().fg(Color::Magenta)),
        Span::raw("] ─ "),
        status_span,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let header_para = Paragraph::new(title).block(block);
    frame.render_widget(header_para, area);
}

fn render_chat(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines = Vec::new();

    for msg in &app.chat_history {
        let (role_label, role_style) = match msg.role {
            MessageRole::User => ("YOU", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            MessageRole::Assistant => ("AI", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            MessageRole::System => ("AI", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        };

        lines.push(Line::from(vec![
            Span::styled(format!("[{}] ", role_label), role_style),
            Span::styled(format!("({}) ", msg.timestamp), Style::default().fg(Color::DarkGray)),
        ]));

        for line in msg.content.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", line))));
        }
        lines.push(Line::raw(""));
    }

    // If currently streaming assistant response
    if !app.current_stream_buffer.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("[AI] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("(streaming...) ", Style::default().fg(Color::DarkGray)),
        ]));
        for line in app.current_stream_buffer.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", line))));
        }
    }

    let block = Block::default()
        .title(" Conversation ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let chat_para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset, 0));

    frame.render_widget(chat_para, area);
}

fn render_accordion(frame: &mut Frame, area: Rect, app: &App) {
    let fallback = "No active reasoning or tool calls yet. (Extended thoughts and ReAct loop events appear here)";
    let content = if !app.current_thinking_buffer.is_empty() {
        app.current_thinking_buffer.as_str()
    } else {
        app.chat_history
            .last()
            .and_then(|m| m.thinking.as_deref())
            .unwrap_or(fallback)
    };

    let lines: Vec<Line> = content
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(Color::DarkGray))))
        .collect();

    let block = Block::default()
        .title(" 🧠 ReAct & Reasoning <think> (Ctrl+T to toggle) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let accordion_para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(accordion_para, area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Prompt (Enter to submit, / for commands) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::White));

    let display_text = format!("> {}", app.input_buffer);
    let para = Paragraph::new(display_text).block(block);
    frame.render_widget(para, area);

    // Set cursor position inside the input block
    let cursor_x = area.x + 3 + app.cursor_idx as u16;
    let cursor_y = area.y + 1;
    if cursor_x < area.x + area.width - 1 {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_slash_palette(frame: &mut Frame, input_area: Rect, app: &App) {
    let height = (app.slash_matches.len().min(8) as u16) + 2;
    let width = 60.min(input_area.width.saturating_sub(4));
    let y = input_area.y.saturating_sub(height);
    let x = input_area.x + 2;

    let popup_area = Rect { x, y, width, height };

    // Clear background
    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = app
        .slash_matches
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let style = if i == app.slash_palette_idx {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let text = format!("/{: <12} {}", cmd.name, cmd.description);
            ListItem::new(Span::styled(text, style))
        })
        .collect();

    let block = Block::default()
        .title(" Commands Autocomplete (Tab/Enter to apply) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);
}

fn render_footer(frame: &mut Frame, area: Rect, _app: &App) {
    let keys = vec![
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Send  "),
        Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
        Span::raw(" Complete  "),
        Span::styled("[Ctrl+T]", Style::default().fg(Color::Yellow)),
        Span::raw(" Toggle Reasoning  "),
        Span::styled("[Esc/Ctrl+C]", Style::default().fg(Color::Red)),
        Span::raw(" Exit"),
    ];

    let footer = Paragraph::new(Line::from(keys));
    frame.render_widget(footer, area);
}
