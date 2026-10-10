use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::theme::UiTheme;
use crate::tui::app::{App, AppStatus, MessageRole};

pub fn render(frame: &mut Frame, app: &mut App) {
    let size = frame.area();
    let theme = UiTheme::default();

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

    render_header(frame, chunks[0], app, &theme);
    render_chat(frame, chunks[1], app, &theme);

    if show_accordion {
        render_accordion(frame, chunks[2], app, &theme);
        render_input(frame, chunks[3], app, &theme);
        render_footer(frame, chunks[4], app, &theme);
        if app.slash_palette_open {
            render_slash_palette(frame, chunks[3], app, &theme);
        }
    } else {
        render_input(frame, chunks[2], app, &theme);
        render_footer(frame, chunks[3], app, &theme);
        if app.slash_palette_open {
            render_slash_palette(frame, chunks[2], app, &theme);
        }
    }

    if app.onboarding_state.is_some() {
        render_onboarding(frame, size, app, &theme);
    } else if app.menu_state.is_open {
        render_interactive_menu(frame, size, app, &theme);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App, theme: &UiTheme) {
    let status_span = match &app.status {
        AppStatus::Idle => Span::styled("● IDLE", Style::default().fg(theme.primary)),
        AppStatus::Thinking => Span::styled("◐ THINKING", Style::default().fg(theme.reasoning).add_modifier(Modifier::BOLD)),
        AppStatus::Streaming => Span::styled("◕ STREAMING", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        AppStatus::Error(e) => Span::styled(format!("✖ ERROR ({})", e), Style::default().fg(theme.error)),
    };

    let grounding_color = match app.grounding_mode.to_lowercase().as_str() {
        "strict" => theme.accent,
        "hybrid" => theme.info,
        "proactive" => theme.magenta,
        _ => theme.text_bright,
    };

    let search_color = match app.search_mode.to_lowercase().as_str() {
        "fast" => theme.primary,
        "deep" => theme.magenta,
        _ => theme.info,
    };

    let (web_text, web_style) = if app.web_search_enabled {
        ("ON", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD))
    } else {
        ("OFF", Style::default().fg(theme.text_muted))
    };

    let (sync_text, sync_style) = if let Some(status) = &app.sync_status {
        if status.is_syncing {
            let spinner_chars = ["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"];
            let spinner = spinner_chars[((app.tick_count * 2) as usize) % spinner_chars.len()];
            (
                format!("⚡ Syncing {} {}", spinner, status.progress_bar),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            )
        } else if status.stage == "cancelled" {
            (
                "🛑 Sync Cancelled".to_string(),
                Style::default().fg(theme.text_muted),
            )
        } else if let Some(ref warn) = app.source_health_warning {
            (
                warn.clone(),
                Style::default().fg(theme.error).add_modifier(Modifier::BOLD),
            )
        } else if app.changes_detected {
            (
                "⚡ Changes detected (/sync)".to_string(),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                "✔ Up to date".to_string(),
                Style::default().fg(theme.primary),
            )
        }
    } else if let Some(ref warn) = app.source_health_warning {
        (
            warn.clone(),
            Style::default().fg(theme.error).add_modifier(Modifier::BOLD),
        )
    } else if app.changes_detected {
        (
            "⚡ Changes detected (/sync)".to_string(),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        )
    } else {
        (
            "✔ Up to date".to_string(),
            Style::default().fg(theme.primary),
        )
    };

    let title = Line::from(vec![
        Span::styled("AnyContext ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(format!("v{} ", env!("CARGO_PKG_VERSION")), Style::default().fg(theme.text_muted)),
        Span::styled("─ [WS: ", Style::default().fg(theme.border_unfocused)),
        Span::styled(&app.active_workspace, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled("] ─ [Sync: ", Style::default().fg(theme.border_unfocused)),
        Span::styled(sync_text, sync_style),
        Span::styled("] ─ [Model: ", Style::default().fg(theme.border_unfocused)),
        Span::styled(&app.active_model, Style::default().fg(theme.reasoning)),
        Span::styled("] ─ [Grounding: ", Style::default().fg(theme.border_unfocused)),
        Span::styled(app.grounding_mode.to_uppercase(), Style::default().fg(grounding_color).add_modifier(Modifier::BOLD)),
        Span::styled("] ─ [Search: ", Style::default().fg(theme.border_unfocused)),
        Span::styled(app.search_mode.to_uppercase(), Style::default().fg(search_color).add_modifier(Modifier::BOLD)),
        Span::styled("] ─ [Web: ", Style::default().fg(theme.border_unfocused)),
        Span::styled(web_text, web_style),
        Span::styled("] ─ ", Style::default().fg(theme.border_unfocused)),
        status_span,
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_unfocused));

    let header_para = Paragraph::new(title).block(block);
    frame.render_widget(header_para, area);
}

fn render_chat(frame: &mut Frame, area: Rect, app: &mut App, theme: &UiTheme) {
    let mut lines = Vec::new();

    for msg in &app.chat_history {
        match msg.role {
            MessageRole::User => {
                lines.push(Line::from(vec![
                    Span::styled("[YOU] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("({}) ", msg.timestamp), Style::default().fg(theme.text_muted)),
                ]));
            }
            MessageRole::Assistant | MessageRole::System => {
                lines.push(Line::from(vec![
                    Span::styled("[AI - ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                    Span::styled(&app.active_workspace, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
                    Span::styled("] ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("({}) ", msg.timestamp), Style::default().fg(theme.text_muted)),
                ]));
            }
        }

        for line in msg.content.lines() {
            lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(theme.text_body))));
        }
        lines.push(Line::raw(""));
    }

    // If currently streaming assistant response
    if !app.current_stream_buffer.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("[AI - ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
            Span::styled(&app.active_workspace, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
            Span::styled("] ", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
            Span::styled("(streaming...) ", Style::default().fg(theme.text_muted)),
        ]));
        for line in app.current_stream_buffer.lines() {
            lines.push(Line::from(Span::styled(format!("  {}", line), Style::default().fg(theme.text_body))));
        }
    }

    // Borderless canvas as requested
    let block = Block::default().borders(Borders::NONE);

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

fn render_accordion(frame: &mut Frame, area: Rect, app: &mut App, theme: &UiTheme) {
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
            Line::from(Span::styled(l, Style::default().fg(theme.reasoning).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("⚡") || trimmed.contains("[ModelRouter: Fast RAG]") {
            Line::from(Span::styled(l, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("• Reason:") || trimmed.contains("Intent:") {
            Line::from(Span::styled(l, Style::default().fg(theme.text_muted)))
        } else if trimmed.starts_with("🌲") || trimmed.contains("[Deep Search: Decomposing") {
            Line::from(Span::styled(l, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("🔄") || trimmed.contains("[Deep Search Iteration") {
            Line::from(Span::styled(l, Style::default().fg(theme.reasoning).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("✔") || trimmed.contains("[Evidence Sufficient]") || trimmed.contains("[Tool Done") {
            Line::from(Span::styled(l, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("🔎") || trimmed.contains("[Gap Analysis") {
            Line::from(Span::styled(l, Style::default().fg(theme.info).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("🔧") || trimmed.contains("[Tool Call:") {
            Line::from(Span::styled(l, Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("❌") || trimmed.contains("[Tool Error:") {
            Line::from(Span::styled(l, Style::default().fg(theme.error).add_modifier(Modifier::BOLD)))
        } else if trimmed.starts_with("1.") || trimmed.starts_with("2.") || trimmed.starts_with("3.") || trimmed.starts_with("4.") || trimmed.starts_with("5.") {
            Line::from(Span::styled(format!("  {}", trimmed), Style::default().fg(theme.text_bright)))
        } else {
            Line::from(Span::styled(l, Style::default().fg(theme.text_muted)))
        };
        lines.push(styled_line);
    }

    let block = Block::default()
        .title(" 🧠 ReAct & Reasoning <think> (Ctrl+T to toggle) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.reasoning));

    let inner_area = block.inner(area);
    let accordion_para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    let total_lines = accordion_para.line_count(inner_area.width);
    let visible_height = inner_area.height as usize;

    app.reasoning_max_scroll = if total_lines > visible_height {
        (total_lines - visible_height) as u16
    } else {
        0
    };

    if app.reasoning_auto_scroll {
        app.reasoning_scroll_offset = app.reasoning_max_scroll;
    } else if app.reasoning_scroll_offset > app.reasoning_max_scroll {
        app.reasoning_scroll_offset = app.reasoning_max_scroll;
    }

    let accordion_para = accordion_para.scroll((app.reasoning_scroll_offset, 0));
    frame.render_widget(accordion_para, area);
}

fn render_input(frame: &mut Frame, area: Rect, app: &App, theme: &UiTheme) {
    let trimmed = app.input_buffer.trim_start();
    let maybe_cmd = if trimmed.starts_with('/') {
        let cmd_word = trimmed[1..].split_whitespace().next().unwrap_or("");
        crate::commands::find_command(cmd_word)
    } else {
        None
    };

    let title_line = if let Some(cmd) = maybe_cmd {
        Line::from(vec![
            Span::styled(" Prompt │ ", Style::default().fg(theme.text_muted)),
            Span::styled(format!("Opções: {} ", cmd.usage), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" Prompt (Enter para enviar, Shift+Enter para nova linha, / para comandos) ", Style::default().fg(theme.text_muted)),
        ])
    };

    let border_color = if maybe_cmd.is_some() {
        theme.border_focus
    } else {
        theme.border_unfocused
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
            Span::styled(prefix, Style::default().fg(theme.accent)),
            Span::styled(*raw_line, Style::default().fg(theme.text_bright)),
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
                                Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC),
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

fn render_slash_palette(frame: &mut Frame, input_area: Rect, app: &App, theme: &UiTheme) {
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
                    .fg(theme.bg_dark)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_bright)
            };

            let text = format!("/{: <11} {: <28} │ {}", cmd.name, cmd.usage, cmd.description);
            ListItem::new(Span::styled(text, style))
        })
        .collect();

    let block = Block::default()
        .title(" Commands Autocomplete (Tab/Enter to apply) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.border_focus));

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App, theme: &UiTheme) {
    let mut spans = Vec::new();

    if let Some(status) = &app.sync_status {
        if status.is_syncing {
            let spinner_chars = ["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"];
            let spinner = spinner_chars[((app.tick_count * 2) as usize) % spinner_chars.len()];
            spans.push(Span::styled(format!("{} Syncing ", spinner), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled(format!("{} ", status.progress_bar), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
        } else if status.stage == "cancelled" {
            spans.push(Span::styled("🛑 Sync cancelled ", Style::default().fg(theme.text_muted)));
            spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
        } else if let Some(ref warn) = app.source_health_warning {
            spans.push(Span::styled(format!("{} ", warn), Style::default().fg(theme.error).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
        } else if app.changes_detected {
            spans.push(Span::styled("⚡ Changes detected (/sync) ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
            spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
        } else {
            spans.push(Span::styled("✔ Up to date ", Style::default().fg(theme.primary)));
            spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
        }
    } else if let Some(ref warn) = app.source_health_warning {
        spans.push(Span::styled(format!("{} ", warn), Style::default().fg(theme.error).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
    } else if app.changes_detected {
        spans.push(Span::styled("⚡ Changes detected (/sync) ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
    } else {
        spans.push(Span::styled("✔ Up to date ", Style::default().fg(theme.primary)));
        spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
    }

    // Scroll Paused indicator
    if !app.auto_scroll && app.scroll_offset < app.max_scroll {
        spans.push(Span::styled("⏸ Scroll Pausado [End p/ tempo real] ", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
        spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
    }

    // Real-time Model Download Telemetry
    if let Some(ref dl) = app.active_model_download {
        let cur_bytes = dl.downloaded_bytes.load(std::sync::atomic::Ordering::Relaxed);
        let percent = if dl.total_bytes > 0 {
            ((cur_bytes as f64 / dl.total_bytes as f64) * 100.0).clamp(0.0, 100.0) as u32
        } else {
            0
        };
        let cur_mb = cur_bytes as f64 / (1024.0 * 1024.0);
        let total_mb = dl.total_bytes as f64 / (1024.0 * 1024.0);
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let spinner = spinner_chars[(app.tick_count as usize) % spinner_chars.len()];

        spans.push(Span::styled(
            format!("📥 Baixando {} {} {:.1}/{:.1} MB ({}%) ", spinner, dl.model_name, cur_mb, total_mb, percent),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled("[Esc p/ cancelar] ", Style::default().fg(theme.reasoning)));
        spans.push(Span::styled("│ ", Style::default().fg(theme.border_unfocused)));
    }

    spans.extend(vec![
        Span::styled("[F1 / /menu]", Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(" Menu  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Enter]", Style::default().fg(theme.primary)),
        Span::styled(" Send  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Tab]", Style::default().fg(theme.accent)),
        Span::styled(" Complete  ", Style::default().fg(theme.text_muted)),
        Span::styled("[PgUp/PgDn]", Style::default().fg(theme.info)),
        Span::styled(" Scroll  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Ctrl+T]", Style::default().fg(theme.reasoning)),
        Span::styled(" Reasoning  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Esc/Ctrl+C]", Style::default().fg(theme.error)),
        Span::styled(" Exit", Style::default().fg(theme.text_muted)),
    ]);

    let footer = Paragraph::new(Line::from(spans));
    frame.render_widget(footer, area);
}

fn render_interactive_menu(frame: &mut Frame, area: Rect, app: &App, theme: &UiTheme) {
    let popup_area = centered_rect(75, 75, area);

    // Clear background so underlying chat is obscured cleanly
    frame.render_widget(Clear, popup_area);

    let breadcrumbs = app.menu_state.breadcrumbs.join(" ➔ ");
    let title = format!(" ⚙️  Menu Interativo ─ [{}] ", breadcrumbs);

    let main_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));

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
        Span::styled("📂 Workspace: ", Style::default().fg(theme.text_muted)),
        Span::styled(&app.active_workspace, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled("  │  🤖 Modelo: ", Style::default().fg(theme.border_unfocused)),
        Span::styled(&app.active_model, Style::default().fg(theme.reasoning).add_modifier(Modifier::BOLD)),
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
                        Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text_muted)
                    },
                ),
                Span::raw(format!("{} ", item.icon)),
                Span::styled(
                    &item.title,
                    if is_selected {
                        Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.text_body)
                    },
                ),
            ];

            if let Some(badge) = &item.badge {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(badge, Style::default().fg(theme.primary)));
            }

            if let Some(shortcut) = &item.shortcut {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(format!("({})", shortcut), Style::default().fg(theme.text_muted)));
            }

            if item.is_submenu {
                spans.push(Span::styled(" ▶", Style::default().fg(theme.accent)));
            }

            let style = if is_selected {
                Style::default().bg(theme.bg_surface)
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
        .border_style(Style::default().fg(theme.border_unfocused))
        .title(" Detalhes ");

    let desc_para = Paragraph::new(format!("ℹ️  {}", desc))
        .block(desc_block)
        .style(Style::default().fg(theme.info));
    frame.render_widget(desc_para, chunks[2]);

    // 4. Footer navigation keys
    let nav_keys = Line::from(vec![
        Span::styled("[↑/↓]", Style::default().fg(theme.accent)),
        Span::styled(" Navegar  •  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Enter/Tab]", Style::default().fg(theme.primary)),
        Span::styled(" Selecionar  •  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Esc/←]", Style::default().fg(theme.text_muted)),
        Span::styled(" Voltar/Fechar", Style::default().fg(theme.text_muted)),
    ]);
    frame.render_widget(Paragraph::new(nav_keys), chunks[3]);
}

fn render_onboarding(frame: &mut Frame, area: Rect, app: &App, theme: &UiTheme) {
    let popup_area = centered_rect(80, 80, area);
    frame.render_widget(Clear, popup_area);

    let onboarding = match &app.onboarding_state {
        Some(s) => s,
        None => return,
    };

    let step_titles = [
        "1. Provedor de IA & Chave de API",
        "2. Perfil de Document AI & Visão",
        "3. Pasta Inicial de Documentos (Opcional)",
    ];

    let current_step_title = step_titles.get(onboarding.step).unwrap_or(&"Configuração");
    let main_title = format!(" 🚀 Assistente de Boas-Vindas AnyContext ─ Passo {}/3: {} ", onboarding.step + 1, current_step_title);

    let main_block = Block::default()
        .title(main_title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent));

    frame.render_widget(main_block.clone(), popup_area);
    let inner_area = main_block.inner(popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Stepper progress bar
            Constraint::Min(6),    // Step content
            Constraint::Length(4), // Contextual tip
            Constraint::Length(1), // Footer keys
        ])
        .split(inner_area);

    // 1. Progress indicator
    let mut step_spans = Vec::new();
    for (i, t) in step_titles.iter().enumerate() {
        if i == onboarding.step {
            step_spans.push(Span::styled(format!(" [● {}] ", t), Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)));
        } else if i < onboarding.step {
            step_spans.push(Span::styled(format!(" [✔ {}] ", t), Style::default().fg(theme.primary)));
        } else {
            step_spans.push(Span::styled(format!(" [○ {}] ", t), Style::default().fg(theme.text_muted)));
        }
        if i < step_titles.len() - 1 {
            step_spans.push(Span::styled("➔", Style::default().fg(theme.border_unfocused)));
        }
    }
    frame.render_widget(Paragraph::new(Line::from(step_spans)), chunks[0]);

    // 2. Step Content
    match onboarding.step {
        0 => {
            let content_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(5),    // Provider list
                    Constraint::Length(3), // API Key input box
                ])
                .split(chunks[1]);

            let providers = [
                ("Google Gemini", "gemini-2.5-flash", "Velocidade otimizada e ampla janela de contexto nativa"),
                ("OpenAI", "gpt-4o-mini", "Raciocínio ágil via ecossistema OpenAI"),
                ("Anthropic Claude", "claude-3-5-sonnet", "Análise aprofundada e precisão analítica"),
                ("Ollama (Local)", "llama3", "Modelos locais auto-hospedados (requer servidor Ollama ativo)"),
                ("Mock Provider", "mock-agent", "Ambiente de desenvolvimento e testes sem consumo de tokens"),
            ];

            let items: Vec<ListItem> = providers
                .iter()
                .enumerate()
                .map(|(i, (name, model, note))| {
                    let is_sel = i == onboarding.selected_provider_idx;
                    let prefix = if is_sel { "▸ " } else { "  " };
                    let style = if is_sel {
                        Style::default().bg(theme.bg_surface)
                    } else {
                        Style::default()
                    };

                    let line = Line::from(vec![
                        Span::styled(prefix, if is_sel { Style::default().fg(theme.accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.text_muted) }),
                        Span::styled(format!("{:<18} ", name), if is_sel { Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.text_body) }),
                        Span::styled(format!("[{}] ", model), Style::default().fg(theme.reasoning)),
                        Span::styled(format!("• {}", note), Style::default().fg(theme.text_muted)),
                    ]);

                    ListItem::new(line).style(style)
                })
                .collect();

            let prov_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if !onboarding.focus_input { Style::default().fg(theme.accent) } else { Style::default().fg(theme.border_unfocused) })
                .title(" 1. Escolha seu Provedor de LLM ([↑/↓] para alternar) ");
            frame.render_widget(List::new(items).block(prov_block), content_chunks[0]);

            let key_title = " 2. Chave de API (Opcional se já configurada via ENV) ";
            let key_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if onboarding.focus_input { Style::default().fg(theme.accent) } else { Style::default().fg(theme.border_unfocused) })
                .title(key_title);

            let masked_key = if onboarding.api_key_input.is_empty() {
                "Digite ou cole sua chave aqui (ou pressione Enter para manter via ENV)...".to_string()
            } else {
                let len = onboarding.api_key_input.len();
                if len > 8 {
                    format!("{}...{}", &onboarding.api_key_input[..4], &onboarding.api_key_input[len - 4..])
                } else {
                    "●".repeat(len)
                }
            };
            let key_style = if onboarding.api_key_input.is_empty() {
                Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC)
            } else {
                Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
            };
            frame.render_widget(Paragraph::new(format!("  🔑 {}", masked_key)).style(key_style).block(key_block), content_chunks[1]);
        }
        1 => {
            // Document AI Profiles - Fully neutral, zero "[Recomendado]" bias
            let profiles = [
                ("Visão Multimodal via Modelo Ativo", "Usa a visão nativa do LLM (Gemini/OpenAI/Claude). Zero download de pesos locais adicionais."),
                ("Extração Nativa Ultraleve", "Extração de texto via bibliotecas nativas de código (PDF, Word, Excel, CSV). 0 MB em disco e inicialização instantânea."),
                ("Pipelines Neurais ONNX Locais", "Modelos ONNX locais para classificação, roteamento semântico e OCR. Baixados sob demanda quando solicitados."),
            ];

            let items: Vec<ListItem> = profiles
                .iter()
                .enumerate()
                .map(|(i, (title, desc))| {
                    let is_sel = i == onboarding.selected_doc_ai_idx;
                    let prefix = if is_sel { "▸ " } else { "  " };
                    let style = if is_sel {
                        Style::default().bg(theme.bg_surface)
                    } else {
                        Style::default()
                    };

                    let line = Line::from(vec![
                        Span::styled(prefix, if is_sel { Style::default().fg(theme.accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.text_muted) }),
                        Span::styled(format!("{:<34} ", title), if is_sel { Style::default().fg(theme.text_bright).add_modifier(Modifier::BOLD) } else { Style::default().fg(theme.text_body) }),
                        Span::styled(format!("• {}", desc), Style::default().fg(theme.text_muted)),
                    ]);

                    ListItem::new(line).style(style)
                })
                .collect();

            let doc_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .title(" Selecione o Perfil de Document AI desejado ([↑/↓] para alternar) ");
            frame.render_widget(List::new(items).block(doc_block), chunks[1]);
        }
        2 => {
            // Initial workspace folder
            let folder_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent))
                .title(" Caminho da Pasta de Documentos Local (Opcional) ");

            let display_path = if onboarding.folder_input.is_empty() {
                "Exemplo: C:\\Users\\SeuUsuario\\Documentos (deixe vazio para configurar depois)...".to_string()
            } else {
                onboarding.folder_input.clone()
            };

            let path_style = if onboarding.folder_input.is_empty() {
                Style::default().fg(theme.text_muted).add_modifier(Modifier::ITALIC)
            } else {
                Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)
            };

            frame.render_widget(Paragraph::new(format!("  📁 {}", display_path)).style(path_style).block(folder_block), chunks[1]);
        }
        _ => {}
    }

    // 3. Contextual Information
    let tip_text = match onboarding.step {
        0 => "Dica: Suas chaves são salvas criptografadas e restritas ao seu usuário local em `%LOCALAPPDATA%\\AnyContext\\anycontext.db`. Você pode alterá-las a qualquer momento em `/keys` ou `/menu`.",
        1 => "Dica: Nenhum download pesado é obrigatório. O AnyContext opera perfeitamente com zero arquivos extras em disco. Modelos neurais locais podem ser baixados a qualquer momento no `/menu`.",
        2 => "Dica: O AnyContext sincroniza seus arquivos em background sem transferir dados privados para servidores externos. Você pode adicionar pastas e URLs a qualquer momento via `/sources`.",
        _ => "",
    };

    let tip_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_unfocused))
        .title(" Informações ");
    frame.render_widget(Paragraph::new(format!("ℹ️  {}", tip_text)).style(Style::default().fg(theme.info)).block(tip_block), chunks[2]);

    // 4. Footer navigation keys
    let nav_spans = vec![
        Span::styled("[Tab]", Style::default().fg(theme.accent)),
        Span::styled(" Alternar Campo  •  ", Style::default().fg(theme.text_muted)),
        Span::styled("[↑/↓]", Style::default().fg(theme.accent)),
        Span::styled(" Selecionar  •  ", Style::default().fg(theme.text_muted)),
        Span::styled("[Enter]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(if onboarding.step < 2 { " Avançar  •  " } else { " Concluir Setup  •  " }, Style::default().fg(theme.text_muted)),
        Span::styled("[Esc]", Style::default().fg(theme.error)),
        Span::styled(if onboarding.step > 0 { " Voltar Passo" } else { " Fechar" }, Style::default().fg(theme.text_muted)),
    ];
    frame.render_widget(Paragraph::new(Line::from(nav_spans)), chunks[3]);
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
