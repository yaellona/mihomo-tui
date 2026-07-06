pub mod actions;
pub mod event;
pub mod msg;
pub mod ui;
pub mod update;

use crate::command::mihomo;
use crate::command::system_proxy::{disable_proxy, enable_proxy, get_proxy_status};
use crate::config::mihomo_config::MihomoConfig;
use crate::config::node::Node;
use crate::constants::{CONFIG_DIR_NAME, CONFIG_FILE, SETTINGS_FILE};
use crate::log::{LogType, Logs};
use crate::settings::Settings;
use ratatui::widgets::TableState;
use std::path::PathBuf;
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
    pub config: MihomoConfig,
    pub config_path: PathBuf,
    pub settings: Settings,
    pub should_quit: bool,
    pub logs: Logs,
    pub log_state: TableState,
    pub url_input: String,
    pub popup_mode: PopupMode,
    pub is_test_delay: bool,
    pub mihomo_running: bool,
    pub async_tx: mpsc::Sender<actions::AsyncTask>,
    pub async_rx: mpsc::Receiver<actions::AsyncTask>,
}

impl App {
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .expect("无法获取配置目录")
            .join(CONFIG_DIR_NAME);
        if !config_dir.exists() {
            let _ = std::fs::create_dir_all(&config_dir);
        }
        let settings_path = config_dir.join(SETTINGS_FILE);
        let config_path = config_dir.join(CONFIG_FILE);

        let settings = Settings::load_or_create(&settings_path);

        let (async_tx, async_rx) = mpsc::channel::<actions::AsyncTask>(settings.channel_capacity);

        let config = MihomoConfig::read_from_file(&config_path).unwrap_or_else(|_| {
            let c = MihomoConfig::default_config(&settings);
            let _ = c.write_to_path(&config_path);
            c
        });

        let mut select_provider = 0;
        if config.proxy_groups.len() > 0 {
            if config.proxy_groups[0].use_list.len() > 0 {
                if let Some(idx) = config
                    .proxy_groups
                    .first()
                    .and_then(|g| g.use_list.first())
                    .and_then(|name| config.provider_index_by_key(name))
                {
                    select_provider = idx;
                }
            }
        }
        let tun_enabled = config.tun.as_ref().map_or(false, |t| t.enable);
        let mihomo_running = mihomo::is_mihomo_running(&settings);

        Self {
            select_node: 0,
            select_provider,
            proxy_running: get_proxy_status().map_or(false, |(v, _)| v == 1),
            tun_enabled,
            active_node: None,
            current_nodes: vec![],
            config,
            config_path,
            settings,
            should_quit: false,
            logs: Logs::new(),
            log_state: TableState::default(),
            url_input: String::new(),
            popup_mode: PopupMode::None,
            is_test_delay: false,
            mihomo_running,
            async_tx,
            async_rx,
        }
    }

    pub fn start_mihomo(&mut self) {
        match mihomo::start_mihomo(&self.settings, &self.config_path) {
            Ok(_) => {
                self.mihomo_running = true;
                self.logs.add_log(LogType::Info, "mihomo启动".to_string());
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }

    pub fn stop_mihomo(&mut self) {
        match mihomo::stop_mihomo(&self.settings) {
            Ok(_) => {
                self.mihomo_running = false;
                self.logs.add_log(LogType::Info, "已停止mihomo".to_string());
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }

    pub fn toggle_mihomo(&mut self) {
        if mihomo::is_mihomo_running(&self.settings) {
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
            match enable_proxy(&format!("127.0.0.1:{}", self.settings.mixed_port)) {
                Ok(_) => self.logs.add_log(LogType::Info, "开启系统代理".to_string()),
                Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
            }
        }
    }

    pub fn toggle_tun(&mut self) {
        let new_state = !self.tun_enabled;
        match self.config.set_tun_enabled(new_state, &self.config_path) {
            Ok(()) => {
                self.tun_enabled = new_state;
                self.logs.add_log(
                    LogType::Info,
                    format!("TUN已{}", if new_state { "开启" } else { "关闭" }),
                );
                #[cfg(unix)]
                if new_state {
                    if let Some(warn) = mihomo::tun_capability_warning() {
                        self.logs.add_log(LogType::Warn, warn);
                    }
                }
                let tx = self.async_tx.clone();
                actions::reload(tx, self.settings.clone(), self.config_path.clone());
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }
}
