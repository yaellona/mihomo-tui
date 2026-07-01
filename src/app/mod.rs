pub mod event;
pub mod ui;
use crate::command::mihomo::Mihomo;
use crate::command::system_proxy::{self, get_proxy_status};
use crate::log::{Log, LogType, Logs};
use crossterm::event::KeyCode;
#[derive(Debug)]
pub struct App {
    pub select_node: usize,
    pub proxy_running: bool,
    pub active_node: Option<usize>,
    pub mihomo: Mihomo,
    pub should_quit: bool,
    pub logs: Logs,
}
impl App {
    pub fn new() -> Self {
        Self {
            select_node: 0,
            proxy_running: get_proxy_status().map_or(false, |(v, _)| v == 1),
            active_node: None,
            mihomo: Mihomo::new("mihomo-windows-amd64.exe".to_string()),
            should_quit: false,
            logs: Logs::new(),
        }
    }
    pub async fn update_node(&mut self) {
        match self.mihomo.update_node().await {
            Ok(_) => self.logs.add_log(LogType::Info, "更新节点".to_string()),
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        };
    }
    pub fn clear(&mut self) {
        match self.mihomo.stop_mihomo() {
            Ok(_) => self.logs.add_log(LogType::Info, "关闭mihomo".to_string()),
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }
    pub fn start_mihomo(&mut self) {
        match self.mihomo.start_mihomo() {
            Ok(_) => self.logs.add_log(LogType::Info, "mihomo启动".to_string()),
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }
    pub async fn test_delay(&mut self) {
        match self.mihomo.test_proxy_delay().await {
            Ok(_) => self.logs.add_log(LogType::Info, "测速成功".to_string()),
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }
    pub async fn on_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            // KeyCode::Char('u') => {
            //     self.popup = PopupMode::UrlInput;
            //     self.url_input.clear();
            // }
            // KeyCode::Char('c') => {
            //     if !self.agencies.is_empty() {
            //         self.popup = PopupMode::AgencySelect;
            //         self.selected_agency = 0;
            //     }
            // }
            // KeyCode::Char('p') => {
            //     self.toggle_system_proxy();
            // }
            KeyCode::Char('t') => {
                self.test_delay().await;
            }
            KeyCode::Up => {
                let len = self.mihomo.current_nodes.len();
                if len > 0 {
                    self.select_node = if self.select_node > 0 {
                        self.select_node - 1
                    } else {
                        len - 1
                    };
                }
            }
            KeyCode::Down => {
                let len = self.mihomo.current_nodes.len();
                if len > 0 {
                    self.select_node = (self.select_node + 1) % len;
                }
            }
            KeyCode::Enter => {
                self.active_node = Some(self.select_node);
                self.proxy_running = true;
                let node_name = self.mihomo.current_nodes[self.select_node].name.clone();

                match self.mihomo.switch_node(&node_name).await {
                    Ok(_) => {
                        self.logs.add_log(LogType::Info, "切换节点".to_string());
                    }
                    Err(e) => {
                        self.logs.add_log(LogType::Error, e.to_string());
                    }
                }
            }
            _ => {}
        }
    }
}
