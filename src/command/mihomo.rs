use crate::config::node::ProxyReport;
use crate::constants::SUBSCRIPTION_UA;
use crate::settings::Settings;
use reqwest;
use std::collections::HashMap;
#[cfg(unix)]
use std::fs;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

// ===== mihomo 进程管理 =====

pub fn start_mihomo(settings: &Settings, config_path: &Path) -> Result<(), String> {
    if is_mihomo_running(settings) {
        return Ok(());
    }
    let config_dir = config_path.parent().ok_or("无法获取配置目录")?;

    let mut cmd = Command::new(&settings.mihomo_exe);
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

pub fn stop_mihomo(settings: &Settings) -> Result<(), String> {
    let image_name = mihomo_image_name(&settings.mihomo_exe);
    kill_all_mihomo(&image_name)
}

// ===== 端口探测 =====

pub fn is_mihomo_running(settings: &Settings) -> bool {
    let addr: std::net::SocketAddr = match settings.mihomo_ctrl_addr.parse() {
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

pub async fn fetch_delays(settings: &Settings) -> Result<HashMap<String, u32>, String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(settings.delay_http_timeout())
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {e}"))?;
    let url = format!(
        "{}/group/Proxy/delay?timeout={}&url={}",
        settings.mihomo_api, settings.delay_timeout_ms, settings.test_url
    );
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

pub async fn reload_config(settings: &Settings, path: PathBuf) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(settings.http_timeout())
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

    let body = serde_json::json!({ "path": path.to_string_lossy(), "payload": "" });
    let url = format!("{}/configs?force=true", settings.mihomo_api);
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

pub async fn get_proxy(settings: &Settings) -> Result<ProxyReport, String> {
    let client: reqwest::Client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/proxies/Proxy", settings.mihomo_api);
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

pub async fn switch_node(settings: &Settings, node_name: String) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/proxies/Proxy", settings.mihomo_api);
    let body = serde_json::json!({ "name": node_name });
    client
        .put(url)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ===== 用flclash的方式来获取代理商的名称 =====

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

pub async fn get_provider_name(settings: &Settings, url: String) -> Result<String, String> {
    let domain = url::Url::parse(&url)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()));
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(settings.delay_http_timeout())
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
