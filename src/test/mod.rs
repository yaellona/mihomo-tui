use crate::command::mihomo::download_mihomo;
use crate::command::system_proxy;
use crate::config::mihomo_config::MihomoConfig;
use crate::settings;
use std::fs;
use tempfile::TempDir;
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_download() {
        let settings = settings::Settings::default();
        download_mihomo(&settings, settings.mihomo_github_url.clone()).await;
    }
}
