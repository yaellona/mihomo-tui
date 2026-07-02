use serde::{Deserialize, Serialize};
use serde_yaml;
use indexmap::IndexMap;
use std::fs;
use std::path::PathBuf;

use crate::constants::{EXTERNAL_CONTROLLER, MIXED_PORT, SOCKS_PORT};

#[derive(Debug, Serialize, Deserialize)]
pub struct MihomoConfig {
    pub port: u16,
    #[serde(rename = "socks-port")]
    pub socks_port: u16,
    #[serde(rename = "allow-lan")]
    pub allow_lan: bool,
    pub mode: String,
    #[serde(rename = "log-level")]
    pub log_level: String,
    #[serde(rename = "external-controller")]
    pub external_controller: String,
    #[serde(rename = "unified-delay")]
    pub unified_delay: bool,
    #[serde(rename = "keep-alive-interval")]
    pub keep_alive_interval: u32,
    #[serde(rename = "clash-for-android")]
    pub clash_for_android: ClashForAndroid,
    pub sniffer: Sniffer,
    #[serde(rename = "proxy-groups")]
    pub proxy_groups: Vec<ProxyGroup>,
    #[serde(rename = "proxy-providers")]
    pub proxy_providers: Option<IndexMap<String, ProxyProvider>>,
    pub rules: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClashForAndroid {
    #[serde(rename = "append-system-dns")]
    pub append_system_dns: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sniffer {
    pub sniff: SniffConfig,
    pub enable: bool,
    #[serde(rename = "force-domain")]
    pub force_domain: Vec<String>,
    #[serde(rename = "skip-domain")]
    pub skip_domain: Vec<String>,
    #[serde(rename = "parse-pure-ip")]
    pub parse_pure_ip: bool,
    #[serde(rename = "force-dns-mapping")]
    pub force_dns_mapping: bool,
    #[serde(rename = "override-destination")]
    pub override_destination: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SniffConfig {
    pub tls: PortConfig,
    pub http: PortConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortConfig {
    pub ports: Vec<String>,
    #[serde(rename = "override-destination")]
    pub override_destination: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyGroup {
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub proxies: Vec<String>,
    #[serde(rename = "use")]
    pub use_list: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyProvider {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub url: String,
    pub interval: u32,
}

impl MihomoConfig {
    pub fn default_config() -> Self {
        Self {
            port: MIXED_PORT,
            socks_port: SOCKS_PORT,
            allow_lan: true,
            mode: "Rule".to_string(),
            log_level: "info".to_string(),
            external_controller: EXTERNAL_CONTROLLER.to_string(),
            unified_delay: true,
            keep_alive_interval: 360,
            clash_for_android: ClashForAndroid {
                append_system_dns: false,
            },
            sniffer: Sniffer {
                sniff: SniffConfig {
                    tls: PortConfig {
                        ports: vec!["1-65535".to_string()],
                        override_destination: true,
                    },
                    http: PortConfig {
                        ports: vec!["1-65535".to_string()],
                        override_destination: true,
                    },
                },
                enable: true,
                force_domain: vec!["+.netflix.com".to_string()],
                skip_domain: vec!["Mijia Cloud".to_string(), "dlg.io.mi.com".to_string()],
                parse_pure_ip: true,
                force_dns_mapping: true,
                override_destination: true,
            },
            proxy_groups: vec![ProxyGroup {
                name: "Proxy".to_string(),
                group_type: "select".to_string(),
                proxies: vec!["DIRECT".to_string()],
                use_list: vec![],
            }],
            proxy_providers: None,
            rules: vec![
                "GEOSITE,category-ads-all,REJECT".to_string(),
                "GEOSITE,google,Proxy".to_string(),
                "GEOSITE,github,Proxy".to_string(),
                "GEOSITE,telegram,Proxy".to_string(),
                "GEOSITE,twitter,Proxy".to_string(),
                "GEOSITE,facebook,Proxy".to_string(),
                "GEOSITE,youtube,Proxy".to_string(),
                "GEOSITE,netflix,Proxy".to_string(),
                "GEOSITE,openai,Proxy".to_string(),
                "GEOIP,LAN,DIRECT".to_string(),
                "GEOIP,CN,DIRECT".to_string(),
                "MATCH,Proxy".to_string(),
            ],
        }
    }

    pub fn insert_sub(&mut self, url: String, config_path: &PathBuf) -> Result<String, String> {
        if self.proxy_providers.is_none() {
            self.proxy_providers = Some(IndexMap::new());
        }
        let hash = format!("{:x}", md5::compute(&url));
        if let Some(ref mut providers) = self.proxy_providers {
            providers.insert(
                hash.to_string(),
                ProxyProvider {
                    provider_type: "http".to_string(),
                    url: url,
                    interval: 3600,
                },
            );
        }
        self.write_to_path(config_path)?;
        Ok(hash)
    }

    pub fn remove_sub(&mut self, sub_name: &str, config_path: &PathBuf) -> Result<(), String> {
        if let Some(ref mut providers) = self.proxy_providers {
            providers.shift_remove(sub_name);
            if providers.is_empty() {
                self.proxy_providers = None;
            }
        }
        self.write_to_path(config_path)?;
        Ok(())
    }

    pub fn from_yaml(yaml_str: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml_str).map_err(|e| format!("解析YAML失败: {}", e))
    }

    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml::to_string(self).map_err(|e| format!("序列化YAML失败: {}", e))
    }

    pub fn read_from_file(config_path: &PathBuf) -> Result<Self, String> {
        let content =
            fs::read_to_string(config_path).map_err(|e| format!("读取文件失败: {}", e))?;

        Self::from_yaml(&content)
    }

    pub fn write_to_path(&self, config_path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
            }
        }
        let yaml_str = self.to_yaml()?;
        fs::write(config_path, yaml_str).map_err(|e| format!("写入文件失败: {}", e))?;
        Ok(())
    }
}
