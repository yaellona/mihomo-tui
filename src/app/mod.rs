pub mod event;
pub mod ui;
use crate::command::mihomo::{self, Mihomo};
use crate::command::system_proxy::{disable_proxy, enable_proxy, get_proxy_status};
use crate::config::node::Node;
use crate::constants::{CHANNEL_CAPACITY, MIXED_PORT, MIHOMO_EXE};
use crate::log::{LogType, Logs};
use crossterm::event::KeyCode;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum PopupMode {
    None,
    UrlInput,
    AgencySelect,
}

pub enum AsyncMsg {
    DelayResult(HashMap<String, u32>), // {节点名: 延迟ms}
    UpdateNode(Vec<Node>),
    SwitchNode,
    SwitchProvider,
    SubChecked {
        sub_name: String,
        err: Option<String>, // None=成功，Some(原因)=失败需回滚
    },
    Error(String),
}

#[derive(Debug)]
pub struct App {
    pub select_node: usize,
    pub select_provider: usize,
    pub proxy_running: bool,
    pub active_node: Option<usize>,
    pub current_nodes: Vec<Node>,
    pub mihomo: Mihomo,
    pub should_quit: bool,
    pub logs: Logs,
    pub url_input: String,
    pub popup_mode: PopupMode,
    pub is_test_speed: bool,
    pub async_tx: mpsc::Sender<AsyncMsg>,
    pub async_rx: mpsc::Receiver<AsyncMsg>,
}
impl App {
    pub fn new() -> Self {
        let (async_tx, async_rx) = mpsc::channel::<AsyncMsg>(CHANNEL_CAPACITY);
        Self {
            select_node: 0,
            select_provider: 0,
            proxy_running: get_proxy_status().map_or(false, |(v, _)| v == 1),
            active_node: None,
            current_nodes: vec![],
            mihomo: Mihomo::new(MIHOMO_EXE.to_string()),
            should_quit: false,
            logs: Logs::new(),
            url_input: String::new(),
            popup_mode: PopupMode::None,
            is_test_speed: false,
            async_tx,
            async_rx,
        }
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

    pub fn toggle_system_proxy(&mut self) {
        let is_enabled = get_proxy_status()
            .map(|(code, _)| code == 1)
            .unwrap_or(false);
        self.proxy_running = !is_enabled;
        if is_enabled {
            // 关闭代理
            match disable_proxy() {
                Ok(_) => self.logs.add_log(LogType::Info, "关闭系统代理".to_string()),
                Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
            };
        } else {
            // 开启代理
            match enable_proxy(&format!("127.0.0.1:{MIXED_PORT}")) {
                Ok(_) => self.logs.add_log(LogType::Info, "开启系统代理".to_string()),
                Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
            };
        }
    }

    // ===== 后台 spawn 包装（App 持有自己的 tx）=====
    pub fn test_delay(&mut self) {
        if self.is_test_speed {
            self.logs.add_log(LogType::Warn, "正在测速！".to_string());
            return;
        }
        self.is_test_speed = true;
        for node in &mut self.current_nodes {
            node.speed = format!("wait");
        }
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            match mihomo::fetch_delays().await {
                Ok(m) => {
                    let _ = tx.send(AsyncMsg::DelayResult(m)).await;
                }
                Err(e) => {
                    let _ = tx.send(AsyncMsg::Error(e)).await;
                }
            }
        });
    }

    pub fn update_nodes(&self) {
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            match mihomo::get_nodes().await {
                Ok(n) => {
                    let _ = tx.send(AsyncMsg::UpdateNode(n)).await;
                }
                Err(e) => {
                    let _ = tx.send(AsyncMsg::Error(e)).await;
                }
            }
        });
    }

    pub fn switch_node(&self, node_name: String) {
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            match mihomo::switch_node(node_name).await {
                Ok(_) => {
                    let _ = tx.send(AsyncMsg::SwitchNode).await;
                }
                Err(e) => {
                    let _ = tx.send(AsyncMsg::Error(e)).await;
                }
            }
        });
    }

    // 需要 &mut self：先在主线程做预处理（改 config + 写盘），再 spawn 后台 reload
    pub fn switch_provider(&mut self, name: String) {
        match self.mihomo.prepare_switch_provider(&name) {
            Ok(path) => {
                self.logs
                    .add_log(LogType::Info, "正在切换代理商...".to_string());
                let tx = self.async_tx.clone();
                tokio::spawn(async move {
                    match mihomo::reload_config(path).await {
                        Ok(_) => {
                            let _ = tx.send(AsyncMsg::SwitchProvider).await;
                        }
                        Err(e) => {
                            let _ = tx.send(AsyncMsg::Error(e)).await;
                        }
                    }
                });
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }

    // 需要 &mut self：先在主线程插入订阅 + 写盘，再 spawn 后台 reload + 校验
    pub fn insert_sub(&mut self, url: String) {
        match self.mihomo.prepare_insert_sub(url) {
            Ok((sub_name, path)) => {
                self.logs
                    .add_log(LogType::Info, "正在验证订阅...".to_string());
                let tx = self.async_tx.clone();
                tokio::spawn(async move {
                    let _ = mihomo::reload_config(path).await; // 先让 mihomo 加载新 provider
                    let err = mihomo::find_provider(sub_name.clone()).await.err(); // 校验
                    let _ = tx.send(AsyncMsg::SubChecked { sub_name, err }).await;
                });
            }
            Err(e) => self.logs.add_log(LogType::Error, e.to_string()),
        }
    }

    // ===== 中央分发：主循环每帧调用 =====
    pub fn poll_msg(&mut self) {
        while let Ok(m) = self.async_rx.try_recv() {
            self.handle_msg(m);
        }
    }

    fn handle_msg(&mut self, m: AsyncMsg) {
        match m {
            AsyncMsg::DelayResult(map) => {
                for node in &mut self.current_nodes {
                    if let Some(&d) = map.get(&node.name) {
                        node.speed = format!("{d}ms");
                    } else {
                        node.speed = format!("-");
                    }
                }
                self.is_test_speed = false;
                self.logs.add_log(LogType::Info, "测速完成".to_string());
            }
            AsyncMsg::UpdateNode(nodes) => {
                self.current_nodes = nodes;
                self.select_node = 0;
                self.logs.add_log(LogType::Info, "更新节点".to_string());
            }
            AsyncMsg::SwitchNode => {
                self.logs.add_log(LogType::Info, "切换节点".to_string());
            }
            AsyncMsg::SwitchProvider => {
                self.popup_mode = PopupMode::None;
                self.logs
                    .add_log(LogType::Info, "切换代理商成功".to_string());
                self.update_nodes();
            }
            AsyncMsg::SubChecked {
                sub_name: _,
                err: None,
            } => {
                self.logs.add_log(LogType::Info, "订阅添加成功".to_string());
                self.update_nodes();
            }
            AsyncMsg::SubChecked {
                sub_name,
                err: Some(e),
            } => {
                // 回滚（需要 &mut self，必须在主线程）
                if let Err(re) = self.mihomo.rollback_sub(&sub_name) {
                    self.logs.add_log(LogType::Error, format!("回滚失败: {re}"));
                }
                let path = self.mihomo.config_path.clone();
                tokio::spawn(async move {
                    let _ = mihomo::reload_config(path).await;
                });
                self.logs
                    .add_log(LogType::Error, format!("订阅失败已回滚: {e}"));
            }
            AsyncMsg::Error(e) => {
                self.logs.add_log(LogType::Error, e);
            }
        }
    }

    pub fn on_key(&mut self, key: KeyCode) {
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
                        self.insert_sub(url);
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
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('p') => self.toggle_system_proxy(),
            KeyCode::Char('c') => self.popup_mode = PopupMode::AgencySelect,
            KeyCode::Char('t') => self.test_delay(),
            KeyCode::Char('r') => self.update_nodes(),
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
                    self.switch_node(node_name);
                }
            }
            _ => {}
        }
    }
}
