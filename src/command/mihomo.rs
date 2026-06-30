use dirs::config_dir;
use reqwest;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};

use crate::config::mihomo_config::MihomoConfig;

pub struct Mihomo {
    child: Option<Child>,
    pub mihomo_path: String,
    pub config_path: PathBuf,
    pub config: MihomoConfig,
}

impl Mihomo {
    pub fn new(mihomo_path: String) -> Self {
        let config_dir = config_dir().ok_or("无法获取配置目录").unwrap();
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("创建目录失败: {}", e))
                .ok();
        }
        let path = config_dir.join("mihomo_config.yaml");
        let config =
            MihomoConfig::read_from_file(&path).unwrap_or_else(|_| MihomoConfig::default_config());
        Self {
            child: None,
            mihomo_path: mihomo_path,
            config_path: path,
            config: config,
        }
    }

    pub fn start_mihomo(&mut self) -> Result<(), String> {
        self.stop_mihomo();

        let config_dir = self
            .config_path
            .parent()
            .ok_or("无法获取配置目录")?;
        let child = Command::new(&self.mihomo_path)
            .args(["-d", config_dir.to_str().ok_or("路径无效")?])
            .spawn()
            .map_err(|e| format!("启动 mihomo 失败: {}", e))?;

        self.child = Some(child);
        Ok(())
    }

    pub fn stop_mihomo(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    pub async fn test_proxy_delay(&self) -> Result<String, reqwest::Error> {
        let url = "http://127.0.0.1:9090/group/Proxy/delay?timeout=5000&url=https://www.gstatic.com/generate_204";
        let body = reqwest::get(url).await?.text().await?;
        Ok(body)
    }
}
