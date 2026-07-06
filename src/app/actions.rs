use crate::app::App;
use crate::app::PopupMode;
use crate::command::mihomo;
use crate::config::node::Node;
use crate::log::LogType;
use crate::settings::Settings;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub type AsyncTask = Box<dyn FnOnce(&mut App) + Send>;

pub fn delay(tx: mpsc::Sender<AsyncTask>, settings: Settings) {
    tokio::spawn(async move {
        let result = mihomo::fetch_delays(&settings).await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(map) => {
                    for node in &mut app.current_nodes {
                        if let Some(&d) = map.get(&node.name) {
                            node.speed = format!("{d}ms");
                        } else {
                            node.speed = "-".to_string();
                        }
                    }
                    app.logs.add_log(LogType::Info, "测速完成".to_string());
                    app.is_test_delay = false;
                }
                Err(e) => {
                    app.is_test_delay = false;
                    app.logs.add_log(LogType::Error, e);
                }
            }))
            .await;
    });
}

pub fn reflash_nodes(tx: mpsc::Sender<AsyncTask>, settings: Settings) {
    tokio::spawn(async move {
        let result = mihomo::get_proxy(&settings).await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(proxy) => {
                    app.current_nodes = vec![];
                    app.select_node = 0;
                    for (index, node) in proxy.all.into_iter().enumerate() {
                        if node == proxy.now {
                            app.active_node = Some(index);
                            app.select_node = index;
                        }
                        app.current_nodes.push(Node::new(node));
                    }
                    app.logs.add_log(LogType::Info, "更新代理信息".to_string());
                }
                Err(e) => {
                    app.logs.add_log(LogType::Error, e);
                }
            }))
            .await;
    });
}

pub fn switch_node(tx: mpsc::Sender<AsyncTask>, settings: Settings, name: String) {
    tokio::spawn(async move {
        let result = mihomo::switch_node(&settings, name.clone()).await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(_) => {
                    app.logs
                        .add_log(LogType::Info, format!("切换节点：{}", name));
                }
                Err(e) => {
                    app.logs.add_log(LogType::Error, e);
                }
            }))
            .await;
    });
}

pub fn reload(tx: mpsc::Sender<AsyncTask>, settings: Settings, path: PathBuf) {
    tokio::spawn(async move {
        let result = mihomo::reload_config(&settings, path).await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(_) => {
                    app.popup_mode = PopupMode::None;
                    app.logs.add_log(LogType::Info, "重置配置成功".to_string());
                    let tx = app.async_tx.clone();
                    reflash_nodes(tx, settings);
                }
                Err(e) => {
                    app.logs.add_log(LogType::Error, e);
                }
            }))
            .await;
    });
}

pub fn insert_sub(tx: mpsc::Sender<AsyncTask>, settings: Settings, url: String) {
    tokio::spawn(async move {
        let result = mihomo::get_provider_name(&settings, url.clone()).await;
        let _ = tx
            .send(Box::new(move |app| {
                let name = match result {
                    Ok(name) => name,
                    Err(e) => {
                        let n = app
                            .config
                            .proxy_providers
                            .as_ref()
                            .map(|p| p.len())
                            .unwrap_or(0)
                            + 1;
                        let fallback = format!("订阅{n}");
                        app.logs
                            .add_log(LogType::Warn, format!("{}，使用默认名称 {}", e, fallback));
                        fallback
                    }
                };
                app.popup_mode = PopupMode::None;

                match app.config.insert_sub(url, name.clone(), &app.config_path) {
                    Ok(_) => {
                        app.logs
                            .add_log(LogType::Info, format!("插入代理商：{}", name));
                        let tx = app.async_tx.clone();
                        reload(tx, settings, app.config_path.clone());
                    }
                    Err(e) => app.logs.add_log(LogType::Error, e),
                }
            }))
            .await;
    });
}
pub fn download_mihomo(app: &mut App, settings: Settings) {
    if app.is_downloading {
        app.logs
            .add_log(LogType::Warn, "正在安装mihomo".to_string());
        return;
    }
    app.is_downloading = true;
    app.logs
        .add_log(LogType::Info, "正在安装mihomo".to_string());
    let tx = app.async_tx.clone();
    tokio::spawn(async move {
        let result = mihomo::download_mihomo(&settings, settings.mihomo_github_url.clone()).await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(_) => {
                    app.logs
                        .add_log(LogType::Info, "下载mihomo成功".to_string());
                    app.is_downloading = false;
                }
                Err(e) => {
                    app.logs.add_log(LogType::Error, e);
                    app.is_downloading = false;
                }
            }))
            .await;
    });
}

impl super::App {
    pub fn navigate_provider(&mut self, step: i32) {
        let len = self
            .config
            .proxy_providers
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(0);
        if len == 0 {
            return;
        }
        self.select_provider = (self.select_provider as i32 + step).rem_euclid(len as i32) as usize;
    }

    pub fn navigate_node(&mut self, step: i32) {
        let len = self.current_nodes.len();
        if len == 0 {
            return;
        }
        self.select_node = (self.select_node as i32 + step).rem_euclid(len as i32) as usize;
    }

    pub fn start_delay_test(&mut self) {
        if self.is_test_delay {
            self.logs
                .add_log(LogType::Warn, "已经在测速了!".to_string());
            return;
        }
        self.is_test_delay = true;
        for node in &mut self.current_nodes {
            node.speed = "wait".to_string();
        }
        let tx = self.async_tx.clone();
        delay(tx, self.settings.clone());
    }

    pub fn delete_current_provider(&mut self) {
        let name = match self.config.provider_key_by_index(self.select_provider) {
            Some(n) => n,
            None => return,
        };
        if let Some(providers) = self.config.proxy_providers.as_mut() {
            providers.shift_remove(&name);
        }
        let _ = self.config.write_to_path(&self.config_path);
        reload(
            self.async_tx.clone(),
            self.settings.clone(),
            self.config_path.clone(),
        );
    }

    pub fn submit_url(&mut self) {
        if self.url_input.is_empty() {
            return;
        }
        let url = self.url_input.clone();
        self.popup_mode = PopupMode::None;
        self.url_input.clear();
        self.logs
            .add_log(LogType::Info, "正在验证URL...".to_string());
        insert_sub(self.async_tx.clone(), self.settings.clone(), url);
    }
}
