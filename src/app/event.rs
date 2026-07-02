use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io;

use crate::constants::POLL_INTERVAL;

pub fn poll_event() -> io::Result<Option<KeyCode>> {
    if event::poll(POLL_INTERVAL)?
        && let Event::Key(key) = event::read()?
        && key.kind == KeyEventKind::Press
    {
        return Ok(Some(key.code));
    }
    Ok(None)
}
