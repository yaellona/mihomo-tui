use crate::app::msg::Msg;
use crate::log::LogType;
use crossterm::event::KeyCode;

use super::PopupMode;
use super::cmd;

impl super::App {
    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Key(k) => self.handle_key(k),
        }
    }

    pub fn poll(&mut self) {
        while let Ok(task) = self.async_rx.try_recv() {
            task(self);
        }
    }

    pub fn load_nodes(&self) {
        let tx = self.async_tx.clone();
        cmd::nodes(tx);
    }

    fn handle_key(&mut self, key: KeyCode) {
        match self.popup_mode {
            PopupMode::UrlInput => {
                match key {
                    KeyCode::Esc => {
                        self.popup_mode = PopupMode::None;
                        self.url_input.clear();
                    }
                    KeyCode::Enter if !self.url_input.is_empty() => {
                        let url = self.url_input.clone();
                        self.popup_mode = PopupMode::None;
                        self.url_input.clear();
                        self.logs
                            .add_log(LogType::Info, "正在验证URL...".to_string());
                        let tx = self.async_tx.clone();
                        // self.insert_sub(url);
                        cmd::insert_sub(tx, url);
                    }
                    KeyCode::Backspace => {
                        self.url_input.pop();
                    }
                    KeyCode::Char(c) => {
                        self.url_input.push(c);
                    }
                    _ => {}
                }
                return;
            }
            PopupMode::AgencySelect => {
                match key {
                    KeyCode::Esc => {
                        self.popup_mode = PopupMode::None;
                    }

                    KeyCode::Up => {
                        let len = self
                            .mihomo
                            .config
                            .proxy_providers
                            .as_ref()
                            .map(|p| p.len())
                            .unwrap_or(0);
                        if len > 0 {
                            self.select_provider = if self.select_provider > 0 {
                                self.select_provider - 1
                            } else {
                                len - 1
                            };
                        }
                    }
                    KeyCode::Down => {
                        let len = self
                            .mihomo
                            .config
                            .proxy_providers
                            .as_ref()
                            .map(|p| p.len())
                            .unwrap_or(0);
                        if len > 0 {
                            self.select_provider = (self.select_provider + 1) % len;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(name) =
                            self.mihomo.get_provider_key_by_index(self.select_provider)
                        {
                            self.switch_provider(name);
                        }
                    }
                    _ => {}
                }
                return;
            }
            PopupMode::None => {}
        }

        match key {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }

            KeyCode::Char('s') => {
                self.toggle_mihomo();
            }

            KeyCode::Char('p') => {
                self.toggle_system_proxy();
            }
            KeyCode::Char('c') => self.popup_mode = PopupMode::AgencySelect,

            KeyCode::Char('t') => {
                if self.is_test_delay {
                    self.logs.add_log(LogType::Warn, format!("已经在测速了!"));
                    return;
                }
                self.is_test_delay = true;
                for node in &mut self.current_nodes {
                    node.speed = "wait".to_string();
                }
                let tx = self.async_tx.clone();
                cmd::delay(tx);
            }
            KeyCode::Char('r') => {
                let tx = self.async_tx.clone();
                cmd::nodes(tx);
            }
            KeyCode::Char('u') => self.popup_mode = PopupMode::UrlInput,
            KeyCode::Up => {
                let len = self.current_nodes.len();
                if len > 0 {
                    self.select_node = if self.select_node > 0 {
                        self.select_node - 1
                    } else {
                        len - 1
                    };
                }
            }
            KeyCode::Down => {
                let len = self.current_nodes.len();
                if len > 0 {
                    self.select_node = (self.select_node + 1) % len;
                }
            }
            KeyCode::Enter => {
                if !self.current_nodes.is_empty() {
                    self.active_node = Some(self.select_node);
                    let node_name = self.current_nodes[self.select_node].name.clone();
                    let tx = self.async_tx.clone();
                    cmd::switch_node(tx, node_name);
                }
            }
            _ => {}
        }
    }

    fn switch_provider(&mut self, name: String) {
        match self.mihomo.prepare_switch_provider(&name) {
            Ok(path) => {
                self.logs
                    .add_log(LogType::Info, "正在切换代理商...".to_string());
                let tx = self.async_tx.clone();
                cmd::reload(tx, path);
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }

    // fn insert_sub(&mut self, url: String) {
    //     match self.mihomo.prepare_insert_sub(url) {
    //         Ok((sub_name, path)) => {
    //             self.logs
    //                 .add_log(LogType::Info, "正在验证订阅...".to_string());
    //             let tx = self.async_tx.clone();
    //             cmd::check_sub(tx, sub_name, path);
    //         }
    //         Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
    //     }
    // }
}
