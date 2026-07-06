use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;

use crate::settings::Settings;

pub fn poll_event(settings: &Settings) -> io::Result<Option<KeyCode>> {
    if event::poll(settings.poll_interval())?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        return Ok(Some(key.code));
    }
    Ok(None)
}
