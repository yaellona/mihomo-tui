use crate::config::mihomo_config::MihomoConfig;
use crate::config::node::{MihomoNodeReport, Node, ProviderReport};
use crate::log::{LogType, Logs};
use dirs::config_dir;
use reqwest;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
pub enum AsyncMsg {
    DelayResult(HashMap<String, u32>), // {节点名: 延迟ms}
    UpdateNode(Vec<Node>),
    SwitchNode(),
    InsertSub(),
    SwitchProvider(),
    Error(String),
}
#[derive(Debug)]
pub struct Mihomo {
    child: Option<Child>,
    pub mihomo_path: String,
    pub config_path: PathBuf,
    pub config: MihomoConfig,
    pub current_nodes: Vec<Node>,
    pub async_tx: mpsc::Sender<AsyncMsg>,
    pub async_rx: mpsc::Receiver<AsyncMsg>,
}

impl Mihomo {
    pub fn new(mihomo_path: String) -> Self {
        let config_dir = config_dir()
            .ok_or("无法获取配置目录")
            .unwrap()
            .join("mihomors");
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("创建目录失败: {}", e))
                .ok();
        }
        let path = config_dir.join("config.yaml");
        let config =
            MihomoConfig::read_from_file(&path).unwrap_or_else(|_| MihomoConfig::default_config());
        let (async_tx, async_rx) = mpsc::channel::<AsyncMsg>(8);
        Self {
            child: None,
            mihomo_path: mihomo_path,
            config_path: path,
            config: config,
            current_nodes: vec![],
            async_rx,
            async_tx,
        }
    }

    pub fn start_mihomo(&mut self) -> Result<(), String> {
        self.stop_mihomo()?;
        let config_dir = self.config_path.parent().ok_or("无法获取配置目录")?;
        let child = Command::new(&self.mihomo_path)
            .args(["-d", config_dir.to_str().ok_or("config路径无效")?])
            .stdout(Stdio::null()) // 隐藏 stdout
            .stderr(Stdio::null()) // 隐藏 stderr
            .spawn()
            .map_err(|e| format!("启动 mihomo 失败: {}", e))?;

        self.child = Some(child);
        Ok(())
    }

    pub fn stop_mihomo(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            child
                .kill()
                .map_err(|e| format!("停止mihomo进程失败: {}", e))?;
            child
                .wait()
                .map_err(|e| format!("等待mihomo进程失败: {}", e))?;
        }
        Ok(())
    }
    pub fn write_config(&self) -> Result<(), String> {
        self.config.write_to_path(&self.config_path)
    }

    pub fn test_delay(&mut self) -> Result<(), String> {
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            match fetch_delays().await {
                Ok(delay) => {
                    let _ = tx.send(AsyncMsg::DelayResult(delay)).await;
                }
                Err(e) => {
                    tx.send(AsyncMsg::Error(e)).await;
                }
            }
        });
        Ok(())
    }
    pub fn switch_node(&mut self, node_name: String) -> Result<(), String> {
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            match switch_node(&node_name).await {
                Ok(_) => {
                    let _ = tx.send(AsyncMsg::SwitchNode()).await;
                }
                Err(e) => {
                    tx.send(AsyncMsg::Error(e.to_string())).await;
                }
            }
        });
        Ok(())
    }
    pub fn update_nodes(&mut self) -> Result<(), String> {
        let tx = self.async_tx.clone();
        tokio::spawn(async move {
            match get_nodes().await {
                Ok(nodes) => {
                    let _ = tx.send(AsyncMsg::UpdateNode(nodes)).await;
                }
                Err(e) => {
                    tx.send(AsyncMsg::Error(e.to_string())).await;
                }
            }
        });
        Ok(())
    }
    pub fn switch_provider(&mut self, provider_name: String) -> Result<(), String> {
        let exists = self
            .config
            .proxy_providers
            .as_ref()
            .map(|providers| providers.contains_key(&provider_name))
            .unwrap_or(false);
        if !exists {
            return Err(format!("代理商 '{}' 不存在", provider_name));
        }
        // 设置 use_list 为选中的 provider 名称
        if let Some(group) = self.config.proxy_groups.first_mut() {
            group.use_list = vec![provider_name];
        }
        self.write_config()?;
        let tx = self.async_tx.clone();
        let config_path = self.config_path.clone();
        tokio::spawn(async move {
            match reload_config(config_path).await {
                Ok(_) => {
                    let _ = tx.send(AsyncMsg::SwitchProvider()).await;
                }
                Err(e) => {
                    tx.send(AsyncMsg::Error(e)).await;
                }
            }
        });
        Ok(())
    }
    pub fn insert_sub(&mut self, url: String) -> Result<(), String> {
        let config_path = self.config_path.clone();
        let sub_name = self.config.insert_sub(url, &self.config_path)?;
        let tx = self.async_tx.clone();
        let mihomo_config = self.config.into();
        tokio::spawn(async move {
            reload_config(config_path).await;

            match find_provider(sub_name).await {
                Ok(_) => {
                    let _ = tx.send(AsyncMsg::SwitchProvider()).await;
                }
                Err(e) => {
                    MihomoConfig::remove_sub(mihomo_config, sub_name);
                    reload_config(config_path).await;
                    tx.send(AsyncMsg::Error(e)).await;
                }
            }
        });
        Ok(())
    }

    fn poll_msg(&mut self, log_manager: Logs) {
        while let Ok(msg) = self.async_rx.try_recv() {
            match msg {
                AsyncMsg::DelayResult(map) => {
                    for node in &mut self.current_nodes {
                        if let Some(&d) = map.get(&node.name) {
                            node.speed = format!("{d}ms");
                        }
                    }
                }
                // AsyncMsg::SwitchNode() => {
                //     // self.logs.add_log(LogType::Info, "切换节点".into());
                // }
                AsyncMsg::UpdateNode(node) => self.current_nodes = node,
                AsyncMsg::Error(e) => {
                    // self.logs.add_log(LogType::Error, e);
                }
                AsyncMsg::InsertSub() => {}
                _ => {}
            }
        }
    }
}

async fn fetch_delays() -> Result<HashMap<String, u32>, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {e}"))?;
    let url = "http://127.0.0.1:9090/group/Proxy/delay?timeout=5000&url=https://www.gstatic.com/generate_204";
    let body = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("测速请求失败: {e}"))?
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {e}"))?;
    serde_json::from_str::<HashMap<String, u32>>(&body).map_err(|e| format!("解析延迟失败: {e}"))
}

async fn reload_config(config_path: PathBuf) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

    let body = serde_json::json!({ "path": config_path, "payload": "" });
    let url = "http://127.0.0.1:9090/configs?force=true";
    let resp = client
        .put(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("重载配置失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("重载配置失败：API返回状态码 {}", resp.status()));
    }
    Ok(())
}
async fn get_nodes() -> Result<Vec<Node>, reqwest::Error> {
    let client: reqwest::Client = reqwest::Client::builder().no_proxy().build()?;
    let url = "http://127.0.0.1:9090/proxies/Proxy";
    let body = client.get(url).send().await?.text().await?;
    let mihomo_report: MihomoNodeReport = serde_json::from_str(&body).unwrap();
    let mut nodes = vec![];
    for node_name in mihomo_report.all {
        nodes.push(Node::new(node_name));
    }
    Ok(nodes)
}

async fn switch_node(node_name: &str) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::builder().no_proxy().build()?;
    let url = "http://127.0.0.1:9090/proxies/Proxy";
    let body = serde_json::json!({"name": node_name});
    client.put(url).json(&body).send().await?;
    Ok(())
}

async fn find_provider(sub_name: String) -> Result<(), String> {
    let provider_url = format!("http://127.0.0.1:9090/providers/proxies/{}", sub_name);
    let mut provider_report: Option<ProviderReport> = None;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
    if let Ok(resp) = client.get(&provider_url).send().await {
        if resp.status().is_success() {
            if let Ok(body) = resp.text().await {
                if let Ok(report) = serde_json::from_str::<ProviderReport>(&body) {
                    provider_report = Some(report);
                }
            }
        }
    }
    let report = provider_report.ok_or("URL验证失败：无法获取provider信息")?;
    let proxy_count = report.proxies.as_ref().map(|p| p.len()).unwrap_or(0);
    if proxy_count == 0 {
        return Err("URL验证失败：无法从该URL加载任何代理节点".to_string());
    }
    Ok(())
}
// pub async fn insert_sub( url: String) -> Result<String, String> {
//     let sub_name = mihomo.config.insert_sub(url, &mihomo.config_path)?;

//     if let Err(e) = mihomo.reload_config().await {
//         let _ = mihomo.config.remove_sub(&sub_name, &mihomo.config_path);
//         return Err(e);
//     }

//     let client = reqwest::Client::builder()
//         .no_proxy()
//         .timeout(Duration::from_secs(5))
//         .build()
//         .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

//     // reload 较快，新 provider 可能尚未拉取完成，带重试等待其就绪。
//     let provider_url = format!("http://127.0.0.1:9090/providers/proxies/{}", sub_name);
//     let mut provider_report: Option<ProviderReport> = None;
//     for _ in 0..6 {
//         if let Ok(resp) = client.get(&provider_url).send().await {
//             if resp.status().is_success() {
//                 if let Ok(body) = resp.text().await {
//                     if let Ok(report) = serde_json::from_str::<ProviderReport>(&body) {
//                         provider_report = Some(report);
//                         break;
//                     }
//                 }
//             }
//         }
//         sleep(Duration::from_millis(500)).await;
//     }

//     let report = provider_report.ok_or("URL验证失败：无法获取provider信息")?;

//     let proxy_count = report.proxies.as_ref().map(|p| p.len()).unwrap_or(0);

//     if proxy_count == 0 {
//         self.config.remove_sub(&sub_name, &self.config_path)?;
//         self.reload_config().await?;
//         return Err("URL验证失败：无法从该URL加载任何代理节点".to_string());
//     }
//     Ok(sub_name)
// }
