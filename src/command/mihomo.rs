use crate::config::mihomo_config::MihomoConfig;
use crate::config::node::{ProviderReport, ProxyReport};
use crate::constants::{
    DELAY_HTTP_TIMEOUT, DELAY_TIMEOUT_MS, HTTP_TIMEOUT, MIHOMO_API, MIHOMO_CTRL_ADDR,
    PROVIDER_RETRY, PROVIDER_RETRY_INTERVAL, TEST_URL,
};
use dirs::config_dir;
use reqwest;
use std::collections::HashMap;
use std::fs;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

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
        Self {
            mihomo_path,
            config_path: path,
            config,
        }
    }

    pub fn start_mihomo(&mut self) -> Result<(), String> {
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
        let pid = find_pid_by_port().ok_or("未找到监听9090端口的mihomo进程")?;
        kill_pid(pid)
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
}

// ===== 端口探测 =====

pub fn is_mihomo_running() -> bool {
    let addr: std::net::SocketAddr = match MIHOMO_CTRL_ADDR.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

// ===== 跨平台：按端口查找 PID =====

#[cfg(unix)]
fn find_pid_by_port() -> Option<u32> {
    // 9090 = 0x2382
    let target_hex = "2382";
    let inode = find_listen_inode("/proc/net/tcp", target_hex)
        .or_else(|| find_listen_inode("/proc/net/tcp6", target_hex))?;
    find_pid_by_inode(inode)
}

#[cfg(unix)]
fn find_listen_inode(path: &str, port_hex: &str) -> Option<u64> {
    let content = fs::read_to_string(path).ok()?;
    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 10 {
            continue;
        }
        // local_address 格式: IP:PORT (十六进制)
        let local = cols[1];
        if !local.ends_with(&format!(":{port_hex}")) {
            continue;
        }
        // state = 0A 表示 LISTEN
        if cols[3] != "0A" {
            continue;
        }
        return cols[9].parse().ok();
    }
    None
}

#[cfg(unix)]
fn find_pid_by_inode(target_inode: u64) -> Option<u32> {
    let proc = fs::read_dir("/proc").ok()?;
    let target = format!("socket:[{target_inode}]");
    for entry in proc.flatten() {
        let name = entry.file_name();
        let pid_str = name.to_string_lossy();
        let pid: u32 = match pid_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let fd_dir = entry.path().join("fd");
        let fds = match fs::read_dir(&fd_dir) {
            Ok(f) => f,
            Err(_) => continue,
        };
        for fd in fds.flatten() {
            if let Ok(link) = fs::read_link(fd.path()) {
                if link.to_string_lossy() == target {
                    return Some(pid);
                }
            }
        }
    }
    None
}

#[cfg(windows)]
fn find_pid_by_port() -> Option<u32> {
    let output = Command::new("netstat").args(["-ano"]).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if !line.contains(":9090") || !line.contains("LISTENING") {
            continue;
        }
        // 最后一列是 PID
        if let Some(pid_str) = line.split_whitespace().last() {
            if let Ok(pid) = pid_str.parse::<u32>() {
                return Some(pid);
            }
        }
    }
    None
}

// ===== 跨平台：杀死进程 =====

#[cfg(unix)]
fn kill_pid(pid: u32) -> Result<(), String> {
    let ret = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    if ret == 0 {
        Ok(())
    } else {
        Err(format!("停止mihomo进程(PID={pid})失败"))
    }
}

#[cfg(windows)]
fn kill_pid(pid: u32) -> Result<(), String> {
    let status = Command::new("taskkill")
        .args(["/F", "/T", "/PID", &pid.to_string()])
        .output()
        .map_err(|e| format!("执行taskkill失败: {e}"))?;
    if status.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&status.stderr);
        Err(format!("停止mihomo进程(PID={pid})失败: {stderr}"))
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
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(DELAY_HTTP_TIMEOUT)
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {e}"))?;
    let resp = client
        .get(url)
        .header("User-Agent", "mihomo")
        .send()
        .await
        .map_err(|e| format!("请求失败: {e}"))?;
    let cd = resp
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    parse_content_disposition(cd).ok_or_else(|| "响应中未找到供应商名称".to_string())
}
