use crate::config::mihomo_config::{Dns, MihomoConfig, Tun};
use crate::config::node::ProxyReport;
use crate::constants::{
    DELAY_HTTP_TIMEOUT, DELAY_TIMEOUT_MS, HTTP_TIMEOUT, MIHOMO_API, MIHOMO_CTRL_ADDR,
    SUBSCRIPTION_UA, TEST_URL,
};
use dirs::config_dir;
use reqwest;
use std::collections::HashMap;
use std::fs;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
// use tokio::time::sleep;

#[derive(Debug)]
pub struct Mihomo {
    pub mihomo_path: String,
    pub config_path: PathBuf,
    pub config: MihomoConfig,
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
        if !path.exists() {
            config.write_to_path(&path).unwrap();
        }
        Self {
            mihomo_path,
            config_path: path,
            config,
        }
    }

    pub fn start_mihomo(&mut self) -> Result<(), String> {
        if is_mihomo_running() {
            return Ok(());
        }
        let config_dir = self.config_path.parent().ok_or("无法获取配置目录")?;

        let mut cmd = Command::new(&self.mihomo_path);
        cmd.args(["-d", config_dir.to_str().ok_or("config路径无效")?])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            const DETACHED_PROCESS: u32 = 0x00000008;
            cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    if libc::setsid() < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        cmd.spawn()
            .map_err(|e| format!("启动 mihomo 失败: {}", e))?;
        Ok(())
    }

    pub fn stop_mihomo(&mut self) -> Result<(), String> {
        let image_name = mihomo_image_name(&self.mihomo_path);
        kill_all_mihomo(&image_name)
    }

    pub fn write_config(&self) -> Result<(), String> {
        self.config.write_to_path(&self.config_path)
    }

    pub fn get_provider_key_by_index(&self, index: usize) -> Option<String> {
        self.config
            .proxy_providers
            .as_ref()
            .and_then(|p| p.keys().nth(index).cloned())
    }
    pub fn get_provider_index_by_key(&self, key: &str) -> Option<usize> {
        self.config
            .proxy_providers
            .as_ref()
            .and_then(|p| p.get_index_of(key))
    }

    pub fn prepare_switch_provider(&mut self, name: &str) -> Result<PathBuf, String> {
        let exists = self
            .config
            .proxy_providers
            .as_ref()
            .map(|providers| providers.contains_key(name))
            .unwrap_or(false);
        if !exists {
            return Err(format!("代理商 '{}' 不存在", name));
        }
        if let Some(group) = self.config.proxy_groups.first_mut() {
            group.use_list = vec![name.to_string()];
        }
        self.write_config()?;
        Ok(self.config_path.clone())
    }

    pub fn set_tun_enabled(&mut self, enabled: bool) -> Result<PathBuf, String> {
        if enabled {
            let tun = self.config.tun.get_or_insert_with(Tun::default_enabled);
            tun.enable = true;
            self.config
                .dns
                .get_or_insert_with(Dns::default_enabled)
                .enable = true;
        } else if let Some(t) = self.config.tun.as_mut() {
            t.enable = false;
        }
        self.write_config()?;
        Ok(self.config_path.clone())
    }
}

// ===== 端口探测 =====

pub fn is_mihomo_running() -> bool {
    let addr: std::net::SocketAddr = match MIHOMO_CTRL_ADDR.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

// ===== 查找 mihomo PID =====

#[cfg(unix)]
fn find_mihomo_pid(config_dir: &str) -> Option<u32> {
    for entry in fs::read_dir("/proc").ok()?.flatten() {
        let pid: u32 = match entry.file_name().to_string_lossy().parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let base = entry.path();
        let comm = match fs::read_to_string(base.join("comm")) {
            Ok(c) => c.trim().to_string(),
            Err(_) => continue,
        };
        if comm != "mihomo" {
            continue;
        }
        if config_dir.is_empty() {
            return Some(pid);
        }
        if let Ok(cmdline) = fs::read(base.join("cmdline")) {
            if String::from_utf8_lossy(&cmdline).contains(config_dir) {
                return Some(pid);
            }
        }
    }
    None
}

#[cfg(unix)]
pub fn tun_capability_warning() -> Option<String> {
    let pid = find_mihomo_pid("")?;
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let cap_eff = status.lines().find_map(|l| {
        l.strip_prefix("CapEff:\t")
            .and_then(|v| u64::from_str_radix(v.trim(), 16).ok())
    })?;
    const CAP_NET_ADMIN: u64 = 1 << 12;
    const CAP_NET_RAW: u64 = 1 << 13;
    if cap_eff & (CAP_NET_ADMIN | CAP_NET_RAW) == (CAP_NET_ADMIN | CAP_NET_RAW) {
        None
    } else {
        Some(format!(
            "mihomo(PID={pid})缺少CAP_NET_ADMIN/CAP_NET_RAW，TUN可能起不来。请用NixOS security.wrappers或setcap授权"
        ))
    }
}

// ===== 跨平台：按映像名杀死所有 mihomo 进程 =====

fn mihomo_image_name(mihomo_path: &str) -> String {
    std::path::Path::new(mihomo_path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| mihomo_path.to_string())
}

#[cfg(windows)]
fn kill_all_mihomo(image_name: &str) -> Result<(), String> {
    let output = Command::new("taskkill")
        .args(["/F", "/T", "/IM", image_name])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("执行taskkill失败: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    if combined.to_lowercase().contains("not found") {
        return Err("未找到 mihomo 进程".to_string());
    }
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("停止mihomo进程({image_name})失败: {combined}"))
    }
}

#[cfg(unix)]
fn kill_all_mihomo(_image_name: &str) -> Result<(), String> {
    let mut killed = 0u32;
    for entry in fs::read_dir("/proc")
        .map_err(|e| format!("读取/proc失败: {e}"))?
        .flatten()
    {
        let pid: u32 = match entry.file_name().to_string_lossy().parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let comm = match fs::read_to_string(entry.path().join("comm")) {
            Ok(c) => c.trim().to_string(),
            Err(_) => continue,
        };
        if comm != "mihomo" {
            continue;
        }
        if unsafe { libc::kill(pid as i32, libc::SIGTERM) } == 0 {
            killed += 1;
        }
    }
    if killed == 0 {
        Err("未找到 mihomo 进程".to_string())
    } else {
        Ok(())
    }
}

// ===== 异步 API 调用 =====

pub async fn fetch_delays() -> Result<HashMap<String, u32>, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(DELAY_HTTP_TIMEOUT)
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {e}"))?;
    let url = format!("{MIHOMO_API}/group/Proxy/delay?timeout={DELAY_TIMEOUT_MS}&url={TEST_URL}");
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

pub async fn reload_config(path: PathBuf) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

    let body = serde_json::json!({ "path": path.to_string_lossy(), "payload": "" });
    let url = format!("{MIHOMO_API}/configs?force=true");
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

pub async fn get_proxy() -> Result<ProxyReport, String> {
    let client: reqwest::Client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{MIHOMO_API}/proxies/Proxy");
    let body = client
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let mihomo_report: ProxyReport =
        serde_json::from_str(&body).map_err(|e| format!("解析节点失败: {e}"))?;
    Ok(mihomo_report)
}

pub async fn switch_node(node_name: String) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{MIHOMO_API}/proxies/Proxy");
    let body = serde_json::json!({ "name": node_name });
    client
        .put(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("xx"),
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).unwrap_or_default()
}

fn parse_content_disposition(cd: &str) -> Option<String> {
    for part in cd.split(';') {
        let p = part.trim();
        if p.to_lowercase().starts_with("filename*=") {
            let val = &p[10..];
            let segs: Vec<&str> = val.split('\'').collect();
            let encoded = if segs.len() >= 3 { segs[2] } else { val };
            return Some(percent_decode(encoded));
        }
    }
    for part in cd.split(';') {
        let p = part.trim();
        if p.to_lowercase().starts_with("filename=") {
            let val = &p[9..];
            return Some(val.trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

pub async fn get_provider_name(url: String) -> Result<String, String> {
    let domain = url::Url::parse(&url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()));
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(DELAY_HTTP_TIMEOUT)
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {e}"))?;
    let resp = client
        .get(&url)
        .header("User-Agent", SUBSCRIPTION_UA)
        .send()
        .await;
    let cd = match &resp {
        Ok(resp) => resp
            .headers()
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .unwrap_or(""),
        Err(e) => {
            if let Some(d) = domain {
                return Ok(d);
            }
            return Err(format!("请求失败: {e}"));
        }
    };
    if let Some(name) = parse_content_disposition(cd) {
        return Ok(name);
    }
    if let Some(d) = domain {
        return Ok(d);
    }
    Err("无法解析订阅名称".to_string())
}
