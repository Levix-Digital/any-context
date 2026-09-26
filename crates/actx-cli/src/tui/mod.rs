pub mod app;
pub mod events;
pub mod ui;
pub mod menu;

use std::io;
use crossterm::{
    cursor::{EnableBlinking, Show},
    event::{Event, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use actx_agent::AgentEvent;

use crate::cli::args::CliArgs;
use crate::engine::{build_agent, resolve_lm_provider};
use self::app::App;
use self::events::handle_key_event;

/// Launches the interactive full-screen TUI session.
pub async fn run_tui(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Resolve Provider and create agent via agnostic engine module
    let effective_ws = args.workspace.clone();
    let (provider, resolved_model) = match resolve_lm_provider(args.model.as_deref(), Some(&effective_ws)) {
        Ok((prov, m)) => (Some(prov), m),
        Err(_) => (None, "mock".to_string()),
    };

    let agent = if let Some(prov) = provider {
        build_agent(prov, &resolved_model, &effective_ws).await.ok()
    } else {
        None
    };

    let mut app = App::new(effective_ws, resolved_model, agent);

    // 2. Setup Terminal in raw mode with alternate screen
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBlinking)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. Event Loop
    let mut reader = EventStream::new();
    let (agent_tx, mut agent_rx) = mpsc::unbounded_channel::<AgentEvent>();

    while app.running {
        terminal.draw(|f| ui::render(f, &mut app))?;

        tokio::select! {
            maybe_event = reader.next() => {
                if let Some(Ok(event)) = maybe_event {
                    if let Event::Key(key) = event {
                        handle_key_event(key, &mut app, &agent_tx);
                    }
                }
            }
            Some(agent_evt) = agent_rx.recv() => {
                app.handle_agent_event(agent_evt);
            }
        }
    }

    // 4. Teardown Terminal cleanly
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)?;
    terminal.show_cursor()?;

    Ok(())
}
