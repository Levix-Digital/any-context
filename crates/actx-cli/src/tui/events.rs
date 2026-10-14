use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use actx_agent::AgentEvent;
use tokio::sync::mpsc;
use crate::tui::app::App;

pub fn handle_key_event(
    key: KeyEvent,
    app: &mut App,
    agent_tx: &mpsc::UnboundedSender<AgentEvent>,
) {
    if key.kind == crossterm::event::KeyEventKind::Release {
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => {
                app.running = false;
                return;
            }
            KeyCode::Char('t') => {
                app.toggle_accordion();
                return;
            }
            KeyCode::Char('m') => {
                if app.menu_state.is_open {
                    app.close_menu();
                } else {
                    app.open_menu();
                }
                return;
            }
            _ => {}
        }
    }

    // F1 toggles interactive menu
    if key.code == KeyCode::F(1) {
        if app.menu_state.is_open {
            app.close_menu();
        } else {
            app.open_menu();
        }
        return;
    }

    // If Onboarding Wizard is active, intercept navigation and input
    if let Some(ref mut onboarding) = app.onboarding_state {
        match key.code {
            KeyCode::Esc => {
                if onboarding.step > 0 {
                    onboarding.step -= 1;
                } else {
                    app.onboarding_state = None;
                }
                return;
            }
            KeyCode::Tab => {
                if onboarding.step == 0 {
                    onboarding.focus_input = !onboarding.focus_input;
                }
                return;
            }
            KeyCode::Up => {
                if onboarding.step == 0 && !onboarding.focus_input {
                    if onboarding.selected_provider_idx > 0 {
                        onboarding.selected_provider_idx -= 1;
                    } else {
                        onboarding.selected_provider_idx = 4;
                    }
                } else if onboarding.step == 1 {
                    if onboarding.selected_doc_ai_idx > 0 {
                        onboarding.selected_doc_ai_idx -= 1;
                    } else {
                        onboarding.selected_doc_ai_idx = 2;
                    }
                }
                return;
            }
            KeyCode::Down => {
                if onboarding.step == 0 && !onboarding.focus_input {
                    if onboarding.selected_provider_idx < 4 {
                        onboarding.selected_provider_idx += 1;
                    } else {
                        onboarding.selected_provider_idx = 0;
                    }
                } else if onboarding.step == 1 {
                    if onboarding.selected_doc_ai_idx < 2 {
                        onboarding.selected_doc_ai_idx += 1;
                    } else {
                        onboarding.selected_doc_ai_idx = 0;
                    }
                }
                return;
            }
            KeyCode::Enter => {
                if onboarding.step < 2 {
                    onboarding.step += 1;
                    onboarding.focus_input = false;
                } else {
                    app.finish_onboarding();
                }
                return;
            }
            KeyCode::Backspace => {
                if onboarding.step == 0 {
                    onboarding.focus_input = true;
                    onboarding.api_key_input.pop();
                } else if onboarding.step == 2 {
                    onboarding.folder_input.pop();
                }
                return;
            }
            KeyCode::Char(c) => {
                if onboarding.step == 0 {
                    onboarding.focus_input = true;
                    onboarding.api_key_input.push(c);
                } else if onboarding.step == 2 {
                    onboarding.folder_input.push(c);
                }
                return;
            }
            _ => {}
        }
        return;
    }

    // If Interactive Menu is open, intercept navigation keys
    if app.menu_state.is_open {
        match key.code {
            KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                app.menu_back();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.menu_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.menu_down();
            }
            KeyCode::Enter | KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                app.menu_select();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            if app.slash_palette_open {
                app.slash_palette_open = false;
            } else if app.is_downloading_model() {
                app.cancel_model_download();
            } else {
                app.running = false;
            }
        }
        KeyCode::Enter => {
            if app.slash_palette_open && app.palette_navigated {
                app.complete_selected_slash(true);
            } else if key.modifiers.contains(KeyModifiers::SHIFT) || key.modifiers.contains(KeyModifiers::ALT) {
                app.insert_char('\n');
            } else {
                app.submit_input(agent_tx.clone());
            }
        }
        KeyCode::Tab => {
            if app.slash_palette_open {
                app.complete_selected_slash(false);
            }
        }
        KeyCode::Up => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.scroll_up(1);
            } else {
                app.palette_up();
            }
        }
        KeyCode::Down => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.scroll_down(1);
            } else {
                app.palette_down();
            }
        }
        KeyCode::PageUp => {
            if app.accordion_open && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.reasoning_auto_scroll = false;
                app.reasoning_scroll_offset = app.reasoning_scroll_offset.saturating_sub(5);
            } else {
                app.scroll_up(10);
            }
        }
        KeyCode::PageDown => {
            if app.accordion_open && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.reasoning_scroll_offset = (app.reasoning_scroll_offset + 5).min(app.reasoning_max_scroll);
                if app.reasoning_scroll_offset >= app.reasoning_max_scroll {
                    app.reasoning_auto_scroll = true;
                }
            } else {
                app.scroll_down(10);
            }
        }
        KeyCode::Home => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.scroll_to_top();
            } else {
                app.cursor_idx = 0;
            }
        }
        KeyCode::End => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                app.scroll_to_bottom();
            } else {
                app.cursor_idx = app.input_buffer.len();
            }
        }
        KeyCode::Left => {
            app.move_cursor_left();
        }
        KeyCode::Right => {
            app.move_cursor_right();
        }
        KeyCode::Backspace => {
            app.delete_backspace();
        }
        KeyCode::Delete => {
            app.delete_forward();
        }
        KeyCode::Char(c) => {
            if c == 'j' && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.insert_char('\n');
            } else if c == '\n' || c == '\r' {
                app.insert_char('\n');
            } else {
                app.insert_char(c);
            }
        }
        _ => {}
    }
}

pub fn handle_mouse_event(mouse: crossterm::event::MouseEvent, app: &mut App) {
    match mouse.kind {
        crossterm::event::MouseEventKind::ScrollUp => {
            app.scroll_up(3);
        }
        crossterm::event::MouseEventKind::ScrollDown => {
            app.scroll_down(3);
        }
        _ => {}
    }
}

