pub mod cmd;
pub mod event;
pub mod msg;
pub mod ui;
pub mod update;

use crate::command::mihomo::{Mihomo, is_mihomo_running};
use crate::command::system_proxy::{disable_proxy, enable_proxy, get_proxy_status};
use crate::config::mihomo_config;
use crate::config::node::Node;
use crate::constants::{CHANNEL_CAPACITY, MIHOMO_EXE, MIXED_PORT};
use crate::log::{LogType, Logs};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum PopupMode {
    None,
    UrlInput,
    AgencySelect,
    HelpKey,
}

#[derive(Debug)]
pub struct App {
    pub select_node: usize,
    pub select_provider: usize,
    pub proxy_running: bool,
    pub tun_enabled: bool,
    pub active_node: Option<usize>,
    pub current_nodes: Vec<Node>,
    pub mihomo: Mihomo,
    pub should_quit: bool,
    pub logs: Logs,
    pub url_input: String,
    pub popup_mode: PopupMode,
    pub is_test_delay: bool,
    pub mihomo_running: bool,
    pub async_tx: mpsc::Sender<cmd::AsyncTask>,
    pub async_rx: mpsc::Receiver<cmd::AsyncTask>,
}

impl App {
    pub fn new() -> Self {
        let (async_tx, async_rx) = mpsc::channel::<cmd::AsyncTask>(CHANNEL_CAPACITY);
        let mihomo = Mihomo::new(MIHOMO_EXE.to_string());
        let mut select_provider = 0;
        if mihomo.config.proxy_groups.len() > 0 {
            if mihomo.config.proxy_groups[0].use_list.len() > 0 {
                if let Some(idx) = mihomo
                    .config
                    .proxy_groups
                    .first()
                    .and_then(|g| g.use_list.first())
                    .and_then(|name| mihomo.get_provider_index_by_key(name))
                {
                    select_provider = idx;
                }
            }
        }
        let tun_enabled = mihomo.config.tun.as_ref().map_or(false, |t| t.enable);

        Self {
            select_node: 0,
            select_provider,
            proxy_running: get_proxy_status().map_or(false, |(v, _)| v == 1),
            tun_enabled,
            active_node: None,
            current_nodes: vec![],
            mihomo,
            should_quit: false,
            logs: Logs::new(),
            url_input: String::new(),
            popup_mode: PopupMode::None,
            is_test_delay: false,
            mihomo_running: is_mihomo_running(),
            async_tx,
            async_rx,
        }
    }

    pub fn start_mihomo(&mut self) {
        match self.mihomo.start_mihomo() {
            Ok(_) => {
                self.mihomo_running = true;
                self.logs.add_log(LogType::Info, "mihomo启动".to_string());
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }

    pub fn stop_mihomo(&mut self) {
        match self.mihomo.stop_mihomo() {
            Ok(_) => {
                self.mihomo_running = false;
                self.logs.add_log(LogType::Info, "已停止mihomo".to_string());
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }

    pub fn toggle_mihomo(&mut self) {
        if is_mihomo_running() {
            self.stop_mihomo();
        } else {
            self.start_mihomo();
            self.load_nodes();
        }
    }

    pub fn toggle_system_proxy(&mut self) {
        let is_enabled = get_proxy_status()
            .map(|(code, _)| code == 1)
            .unwrap_or(false);
        self.proxy_running = !is_enabled;
        if is_enabled {
            match disable_proxy() {
                Ok(_) => self.logs.add_log(LogType::Info, "关闭系统代理".to_string()),
                Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
            };
        } else {
            match enable_proxy(&format!("127.0.0.1:{MIXED_PORT}")) {
                Ok(_) => self.logs.add_log(LogType::Info, "开启系统代理".to_string()),
                Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
            };
        }
    }

    pub fn toggle_tun(&mut self) {
        let new_state = !self.tun_enabled;
        match self.mihomo.set_tun_enabled(new_state) {
            Ok(path) => {
                self.tun_enabled = new_state;
                self.logs.add_log(
                    LogType::Info,
                    format!("TUN已{}", if new_state { "开启" } else { "关闭" }),
                );
                #[cfg(unix)]
                if new_state {
                    if let Some(warn) = crate::command::mihomo::tun_capability_warning() {
                        self.logs.add_log(LogType::Warn, warn);
                    }
                }
                let tx = self.async_tx.clone();
                cmd::reload(tx, path);
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }
}
