//! 全局常量集中定义，避免散落在各文件的魔法值。

pub const MIHOMO_API: &str = "http://127.0.0.1:9090";
pub const MIXED_PORT: u16 = 7890;
pub const SOCKS_PORT: u16 = 7891;
pub const TEST_URL: &str = "https://www.gstatic.com/generate_204";
pub const DELAY_TIMEOUT_MS: u64 = 5000;
pub const EXTERNAL_CONTROLLER: &str = ":9090";

pub const HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
pub const DELAY_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(6);
pub const PROVIDER_RETRY: u32 = 6;
pub const PROVIDER_RETRY_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

pub const MIHOMO_EXE: &str = "mihomo-windows-amd64.exe";
pub const CHANNEL_CAPACITY: usize = 16;
