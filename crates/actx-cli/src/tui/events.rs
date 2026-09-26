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
            _ => {}
        }
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
            if app.slash_palette_open {
                app.complete_selected_slash();
            } else {
                app.submit_input(agent_tx.clone());
            }
        }
        KeyCode::Tab => {
            if app.slash_palette_open {
                app.complete_selected_slash();
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
