use crossterm::event::KeyCode;

#[derive(Debug)]

pub enum Msg {
    Key(KeyCode),
}
