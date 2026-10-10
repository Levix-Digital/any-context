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

    // Dynamic Input Height Calculation (clamped to 3..8 lines)
    let input_inner_width = size.width.saturating_sub(6).max(20) as usize;
    let mut total_input_lines: usize = 0;
    for l in app.input_buffer.split('\n') {
        let chars = l.chars().count();
        let rows = if chars == 0 { 1 } else { (chars + input_inner_width - 1) / input_inner_width };
        total_input_lines += rows.max(1);
    }
    let input_height = ((total_input_lines.max(1) + 2) as u16).clamp(3, 8);

    // Vertical Layout
    let chunks = if show_accordion {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(6),    // Chat viewport
                Constraint::Length(7), // Accordion (thinking & tool logs)
                Constraint::Length(input_height), // Dynamic Input area
                Constraint::Length(1), // Footer
            ])
            .split(size)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(8),    // Chat viewport
                Constraint::Length(input_height), // Dynamic Input area
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

    if app.menu_state.is_open {
        render_interactive_menu(frame, size, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let status_span = match &app.status {
        AppStatus::Idle => Span::styled("● IDLE", Style::default().fg(Color::Green)),
        AppStatus::Thinking => Span::styled("◐ THINKING", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        AppStatus::Streaming => Span::styled("◕ STREAMING", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        AppStatus::Error(e) => Span::styled(format!("✖ ERROR ({})", e), Style::default().fg(Color::Red)),
    };

    let grounding_color = match app.grounding_mode.to_lowercase().as_str() {
        "strict" => Color::Blue,
        "hybrid" => Color::Cyan,
        "proactive" => Color::LightMagenta,
        _ => Color::White,
    };

    let (web_text, web_style) = if app.web_search_enabled {
        ("ON", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        ("OFF", Style::default().fg(Color::DarkGray))
    };

    let (sync_text, sync_style) = if let Some(status) = &app.sync_status {
        if status.is_syncing {
            let spinner_chars = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
            let spinner = spinner_chars[(app.tick_count as usize) % spinner_chars.len()];
            (
                format!("⚡ Syncing {} {}", spinner, status.progress_bar),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )
        } else if status.stage == "cancelled" {
            (
                "🛑 Sync Cancelled".to_string(),
                Style::default().fg(Color::Red),
            )
        } else {
            (
                "✔ Up to date".to_string(),
                Style::default().fg(Color::Green),
            )
        }
    } else {
        (
            "✔ Up to date".to_string(),
            Style::default().fg(Color::Green),
        )
    };

    let title = Line::from(vec![
        Span::styled("AnyContext ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(format!("v{} ", env!("CARGO_PKG_VERSION")), Style::default().fg(Color::DarkGray)),
        Span::raw("─ [WS: "),
        Span::styled(&app.active_workspace, Style::default().fg(Color::Yellow)),
        Span::raw("] ─ [Sync: "),
        Span::styled(sync_text, sync_style),
        Span::raw("] ─ [Model: "),
        Span::styled(&app.active_model, Style::default().fg(Color::Magenta)),
        Span::raw("] ─ [Grounding: "),
        Span::styled(app.grounding_mode.to_uppercase(), Style::default().fg(grounding_color).add_modifier(Modifier::BOLD)),
        Span::raw("] ─ [Web: "),
        Span::styled(web_text, web_style),
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

fn render_chat(frame: &mut Frame, area: Rect, app: &mut App) {
    let mut lines = Vec::new();

    for msg in &app.chat_history {
        match msg.role {
            MessageRole::User => {
                lines.push(Line::from(vec![
                    Span::styled("[YOU] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("({}) ", msg.timestamp), Style::default().fg(Color::DarkGray)),
                ]));
            }
            MessageRole::Assistant | MessageRole::System => {
                lines.push(Line::from(vec![
                    Span::styled("[AI - ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(&app.active_workspace, Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
                    Span::styled("] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("({}) ", msg.timestamp), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        for line in msg.content.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", line))));
        }
        lines.push(Line::raw(""));
    }

    // If currently streaming assistant response
    if !app.current_stream_buffer.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("[AI - ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(&app.active_workspace, Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
            Span::styled("] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("(streaming...) ", Style::default().fg(Color::DarkGray)),
        ]));
        for line in app.current_stream_buffer.lines() {
            lines.push(Line::from(Span::raw(format!("  {}", line))));
        }
    }

    let title_text = if !app.auto_scroll && app.scroll_offset < app.max_scroll {
        " Conversation [Scroll Paused - PgDn/End to auto-scroll] "
    } else {
        " Conversation "
    };

    let block = Block::default()
        .title(title_text)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    let chat_para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    let total_lines = chat_para.line_count(inner_area.width);
    let visible_height = inner_area.height as usize;

    app.max_scroll = if total_lines > visible_height {
        (total_lines - visible_height) as u16
    } else {
        0
    };

    if app.auto_scroll {
        app.scroll_offset = app.max_scroll;
    } else if app.scroll_offset > app.max_scroll {
        app.scroll_offset = app.max_scroll;
    }

    let chat_para = chat_para.scroll((app.scroll_offset, 0));
    frame.render_widget(chat_para, area);
}


fn render_accordion(frame: &mut Frame, area: Rect, app: &App) {
    let fallback = "No active reasoning or tool calls yet. (Extended thoughts and ReAct loop events appear here)";
    let content = if !app.current_thinking_buffer.is_empty() {
        app.current_thinking_buffer.as_str()
    } else {
        app.chat_history
            .iter()
            .rev()
            .find_map(|m| m.thinking.as_deref())
            .unwrap_or(fallback)
    };

    let mut lines: Vec<Line> = Vec::new();
    for l in content.lines() {
        let trimmed = l.trim();
        let styled_line = if trimmed.starts_with("🧠") || trimmed.contains("[ModelRouter: Deep Search]") {
            Line::from(Span::styled(l, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("⚡") || trimmed.contains("[ModelRouter: Fast RAG]") {
            Line::from(Span::styled(l, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("• Reason:") || trimmed.contains("Intent:") {
            Line::from(Span::styled(l, Style::default().fg(Color::Gray)))
        } else if trimmed.starts_with("🌲") || trimmed.contains("[Deep Search: Decomposing") {
            Line::from(Span::styled(l, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("🔄") || trimmed.contains("[Deep Search Iteration") {
            Line::from(Span::styled(l, Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("✔") || trimmed.contains("[Evidence Sufficient]") || trimmed.contains("[Tool Done") {
            Line::from(Span::styled(l, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("🔎") || trimmed.contains("[Gap Analysis") {
            Line::from(Span::styled(l, Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("🔧") || trimmed.contains("[Tool Call:") {
            Line::from(Span::styled(l, Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("❌") || trimmed.contains("[Tool Error:") {
            Line::from(Span::styled(l, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("1.") || trimmed.starts_with("2.") || trimmed.starts_with("3.") || trimmed.starts_with("4.") || trimmed.starts_with("5.") {
            Line::from(Span::styled(format!("  {}", trimmed), Style::default().fg(Color::White)))
        } else {
            Line::from(Span::styled(l, Style::default().fg(Color::DarkGray)))
        };
        lines.push(styled_line);
    }

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
    let trimmed = app.input_buffer.trim_start();
    let maybe_cmd = if trimmed.starts_with('/') {
        let cmd_word = trimmed[1..].split_whitespace().next().unwrap_or("");
        crate::commands::find_command(cmd_word)
    } else {
        None
    };

    let title_line = if let Some(cmd) = maybe_cmd {
        Line::from(vec![
            Span::raw(" Prompt │ "),
            Span::styled(format!("Opções: {} ", cmd.usage), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(vec![
            Span::raw(" Prompt (Enter para enviar, Shift+Enter para nova linha, / para comandos) "),
        ])
    };

    let border_color = if maybe_cmd.is_some() {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(title_line)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let raw_lines: Vec<&str> = app.input_buffer.split('\n').collect();
    let mut input_lines: Vec<Line> = Vec::new();

    for (idx, raw_line) in raw_lines.iter().enumerate() {
        let prefix = if idx == 0 { "> " } else { "  " };
        let mut spans = vec![
            Span::styled(prefix, Style::default().fg(Color::DarkGray)),
            Span::styled(*raw_line, Style::default().fg(Color::Gray)),
        ];

        // Contextual inline ghost text on the last line for expected parameters
        if idx == raw_lines.len().saturating_sub(1) {
            if let Some(cmd) = maybe_cmd {
                if app.cursor_idx >= app.input_buffer.len() {
                    let cmd_prefix = format!("/{}", cmd.name);
                    let raw_trimmed = app.input_buffer.trim();
                    let matches_cmd_name = raw_trimmed.eq_ignore_ascii_case(&cmd_prefix)
                        || cmd.aliases.iter().any(|a| raw_trimmed.eq_ignore_ascii_case(&format!("/{}", a)));

                    if matches_cmd_name {
                        let usage_params = cmd.usage.split_once(' ').map(|(_, p)| p).unwrap_or("");
                        if !usage_params.is_empty() {
                            let ghost = if app.input_buffer.ends_with(' ') {
                                usage_params.to_string()
                            } else {
                                format!(" {}", usage_params)
                            };
                            spans.push(Span::styled(
                                ghost,
                                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
                            ));
                        }
                    }
                }
            }
        }
        input_lines.push(Line::from(spans));
    }

    let para = Paragraph::new(input_lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);

    // 2D Cursor placement calculation with newline & width-wrap awareness
    let inner_width = area.width.saturating_sub(4).max(10) as usize;
    let mut cursor_row: u16 = 0;
    let mut cursor_col: u16 = 0;
    let mut char_count_so_far = 0;

    let split_lines: Vec<&str> = app.input_buffer.split('\n').collect();
    for (line_idx, line_str) in split_lines.iter().enumerate() {
        let line_len = line_str.chars().count();
        let is_last = line_idx == split_lines.len().saturating_sub(1);
        if app.cursor_idx <= char_count_so_far + line_len || is_last {
            let line_cursor_pos = app.cursor_idx.saturating_sub(char_count_so_far).min(line_len);
            let wrapped_row = line_cursor_pos / inner_width;
            let wrapped_col = line_cursor_pos % inner_width;
            cursor_row += wrapped_row as u16;
            cursor_col = wrapped_col as u16;
            break;
        } else {
            let wrapped_rows = if line_len == 0 { 1 } else { (line_len + inner_width - 1) / inner_width };
            cursor_row += wrapped_rows.max(1) as u16;
            char_count_so_far += line_len + 1; // account for '\n'
        }
    }

    let cursor_x = area.x + 3 + cursor_col;
    let cursor_y = area.y + 1 + cursor_row;
    if cursor_y < area.y + area.height - 1 && cursor_x < area.x + area.width - 1 {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_slash_palette(frame: &mut Frame, input_area: Rect, app: &App) {
    let height = (app.slash_matches.len().min(8) as u16) + 2;
    let width = 75.min(input_area.width.saturating_sub(4));
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

            let text = format!("/{: <11} {: <28} │ {}", cmd.name, cmd.usage, cmd.description);
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

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = Vec::new();

    if let Some(status) = &app.sync_status {
        if status.is_syncing {
            let spinner_chars = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
            let spinner = spinner_chars[(app.tick_count as usize) % spinner_chars.len()];
            spans.push(Span::styled(format!("{} Syncing ", spinner), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(format!("{} ", status.progress_bar), Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
        } else if status.stage == "cancelled" {
            spans.push(Span::styled("🛑 Sync cancelled ", Style::default().fg(Color::Red)));
            spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
        } else {
            spans.push(Span::styled("✔ Up to date ", Style::default().fg(Color::Green)));
            spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
        }
    } else {
        spans.push(Span::styled("✔ Up to date ", Style::default().fg(Color::Green)));
        spans.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
    }

    spans.extend(vec![
        Span::styled("[F1 / /menu]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" Menu  "),
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Send  "),
        Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
        Span::raw(" Complete  "),
        Span::styled("[PgUp/PgDn]", Style::default().fg(Color::Cyan)),
        Span::raw(" Scroll  "),
        Span::styled("[Ctrl+T]", Style::default().fg(Color::Yellow)),
        Span::raw(" Reasoning  "),
        Span::styled("[Esc/Ctrl+C]", Style::default().fg(Color::Red)),
        Span::raw(" Exit"),
    ]);

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}

fn render_interactive_menu(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(75, 75, area);

    // Clear background so underlying chat is obscured cleanly
    frame.render_widget(Clear, popup_area);

    let breadcrumbs = app.menu_state.breadcrumbs.join(" ➔ ");
    let title = format!(" ⚙️  Menu Interativo ─ [{}] ", breadcrumbs);

    let main_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    frame.render_widget(main_block.clone(), popup_area);

    let inner_area = main_block.inner(popup_area);

    // Inner layout: Header bar (1), Items list (min 4), Description box (3), Footer (1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Sub-header / active context info
            Constraint::Min(4),    // Items list
            Constraint::Length(3), // Selected item details
            Constraint::Length(1), // Key bindings
        ])
        .split(inner_area);

    // 1. Sub-header
    let header_line = Line::from(vec![
        Span::raw("📂 Workspace: "),
        Span::styled(&app.active_workspace, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("  │  🤖 Modelo: "),
        Span::styled(&app.active_model, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
    ]);
    frame.render_widget(Paragraph::new(header_line), chunks[0]);

    // 2. Items list
    let items: Vec<ListItem> = app
        .menu_state
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.menu_state.selected_idx;
            let prefix = if is_selected { "▸ " } else { "  " };

            let mut spans = vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::raw(format!("{} ", item.icon)),
                Span::styled(
                    &item.title,
                    if is_selected {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    },
                ),
            ];

            if let Some(badge) = &item.badge {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(badge, Style::default().fg(Color::Green)));
            }

            if let Some(shortcut) = &item.shortcut {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(format!("({})", shortcut), Style::default().fg(Color::DarkGray)));
            }

            if item.is_submenu {
                spans.push(Span::styled(" ▶", Style::default().fg(Color::Cyan)));
            }

            let style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(spans)).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, chunks[1]);

    // 3. Selected item details
    let desc = app
        .menu_state
        .selected_item()
        .map(|it| it.description.as_str())
        .unwrap_or("Selecione uma opção com [Enter] ou navegue com [↑/↓]");

    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Detalhes ");

    let desc_para = Paragraph::new(format!("ℹ️  {}", desc))
        .block(desc_block)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(desc_para, chunks[2]);

    // 4. Footer navigation keys
    let nav_keys = Line::from(vec![
        Span::styled("[↑/↓]", Style::default().fg(Color::Yellow)),
        Span::raw(" Navegar  •  "),
        Span::styled("[Enter/Tab]", Style::default().fg(Color::Green)),
        Span::raw(" Selecionar  •  "),
        Span::styled("[Esc/←]", Style::default().fg(Color::Red)),
        Span::raw(" Voltar/Fechar"),
    ]);
    frame.render_widget(Paragraph::new(nav_keys), chunks[3]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
