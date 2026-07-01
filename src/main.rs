mod app;
mod command;
mod config;
mod log;
#[cfg(test)]
mod test;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use std::time::Duration;
use tokio::time::Sleep;

use ratatui::{Terminal, backend::CrosstermBackend};
use std::collections::HashMap;
use std::io;
#[tokio::main]
async fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = app::App::new();
    app.mihomo.start_mihomo();
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    app.update_node().await;
    loop {
        terminal.draw(|f| app::ui::draw(f, &app))?;

        if let Some(key) = app::event::poll_event()? {
            app.on_key(key).await;
        }
        if app.should_quit {
            break;
        }
    }
    let _ = app.mihomo.stop_mihomo();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
