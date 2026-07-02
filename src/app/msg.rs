use crate::config::node::Node;
use crossterm::event::KeyCode;
use std::collections::HashMap;

#[derive(Debug)]
pub enum Msg {
    Key(KeyCode),
    Delay(HashMap<String, u32>),
    Nodes(Vec<Node>),
    SwitchedNode,
    SwitchedProvider,
    SubChecked {
        sub_name: String,
        err: Option<String>, // None=成功，Some(原因)=失败需回滚
    },
    Error(String),
}
