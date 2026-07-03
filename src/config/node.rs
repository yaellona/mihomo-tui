use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub speed: String,
}
impl Node {
    pub fn new(name: String) -> Self {
        Self {
            name: name,
            speed: "-".to_string(),
        }
    }
}

//用于接收mihomo的回复用的节点
#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyReport {
    pub alive: bool,
    pub all: Vec<String>,
    #[serde(rename = "dialer-proxy")]
    pub dialer_proxy: String,
    pub hidden: bool,
    pub icon: String,
    pub interface: String,
    pub name: String,
    pub now: String,
    #[serde(rename = "type")]
    pub node_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyItem {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub server: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderReport {
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub proxies: Option<Vec<ProxyItem>>,
    #[serde(rename = "vehicleType")]
    pub vehicle_type: Option<String>,
    #[serde(rename = "subscriptionInfo")]
    pub subscription_info: Option<serde_json::Value>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    #[serde(rename = "healthCheck")]
    pub health_check: Option<serde_json::Value>,
}
