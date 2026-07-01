use dirs::config_dir;
use reqwest;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use crate::config::mihomo_config::MihomoConfig;
use crate::config::node::{MihomoNodeReport, Node};
#[derive(Debug)]
pub struct Mihomo {
    child: Option<Child>,
    pub mihomo_path: String,
    pub config_path: PathBuf,
    pub config: MihomoConfig,
    pub current_nodes: Vec<Node>,
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
        Self {
            child: None,
            mihomo_path: mihomo_path,
            config_path: path,
            config: config,
            current_nodes: vec![],
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
    pub async fn test_proxy_delay(&mut self) -> Result<(), reqwest::Error> {
        let client = reqwest::Client::builder().no_proxy().build()?;
        let url = "http://127.0.0.1:9090/group/Proxy/delay?timeout=5000&url=https://www.gstatic.com/generate_204";
        let body = client.get(url).send().await?.text().await?;
        let delays: HashMap<String, u32> = serde_json::from_str(&body).unwrap();
        for node in &mut self.current_nodes {
            if let Some(&delay) = delays.get(&node.name) {
                node.speed = format!("{}ms", delay);
            }
        }
        Ok(())
    }
    pub fn write_config(&self) -> Result<(), String> {
        self.config.write_to_path(&self.config_path)
    }
    pub fn insert_sub(&mut self, url: String) -> Result<String, String> {
        self.config.insert_sub(url, &self.config_path)
    }
    pub async fn update_node(&mut self) -> Result<(), reqwest::Error> {
        let client: reqwest::Client = reqwest::Client::builder().no_proxy().build()?;
        let url = "http://127.0.0.1:9090/proxies/Proxy";
        let body = client.get(url).send().await?.text().await?;
        let mihomo_report: MihomoNodeReport = serde_json::from_str(&body).unwrap();
        self.current_nodes.clear();
        for node_name in mihomo_report.all {
            self.current_nodes.push(Node::new(node_name));
        }
        Ok(())
    }
    pub async fn switch_node(&mut self, node_name: &str) -> Result<(), reqwest::Error> {
        let client = reqwest::Client::builder().no_proxy().build()?;
        let url = "http://127.0.0.1:9090/proxies/Proxy";
        let body = serde_json::json!({"name": node_name});
        client.put(url).json(&body).send().await?;
        Ok(())
    }
}
