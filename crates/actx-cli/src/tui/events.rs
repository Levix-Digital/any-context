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
            } else {
                app.running = false;
            }
        }
        KeyCode::Enter => {
            if app.slash_palette_open && app.palette_navigated {
                app.complete_selected_slash();
                app.palette_navigated = false;
            } else {
                app.submit_input(agent_tx.clone());
            }
        }
        KeyCode::Tab => {
            if app.slash_palette_open {
                app.complete_selected_slash();
                app.palette_navigated = false;
            }
        }
        KeyCode::Up => {
            app.palette_up();
        }
        KeyCode::Down => {
            app.palette_down();
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
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        _ => {}
    }
}
