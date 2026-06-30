use crate::command::mihomo::Mihomo;
use crate::command::system_proxy;
use crate::config::mihomo_config::MihomoConfig;
use std::fs;
use tempfile::TempDir;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MihomoConfig::default_config();
        assert_eq!(config.port, 7890);
        assert_eq!(config.socks_port, 7891);
        assert!(config.allow_lan);
        assert_eq!(config.mode, "Rule");
        assert_eq!(config.log_level, "info");
        assert_eq!(config.external_controller, ":9090");
        assert!(config.unified_delay);
        assert_eq!(config.keep_alive_interval, 360);
        assert!(!config.clash_for_android.append_system_dns);
        assert!(config.sniffer.enable);
        assert_eq!(config.proxy_groups.len(), 1);
        assert_eq!(config.proxy_groups[0].name, "Proxy");
        assert!(config.proxy_providers.is_none());
        assert_eq!(config.rules.len(), 12);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let config = MihomoConfig::default_config();
        let yaml = config.to_yaml().expect("序列化YAML失败");
        let config2 = MihomoConfig::from_yaml(&yaml).expect("反序列化YAML失败");
        assert_eq!(config.port, config2.port);
        assert_eq!(config.socks_port, config2.socks_port);
        assert_eq!(config.allow_lan, config2.allow_lan);
        assert_eq!(config.mode, config2.mode);
        assert_eq!(config.log_level, config2.log_level);
        assert_eq!(config.external_controller, config2.external_controller);
        assert_eq!(config.unified_delay, config2.unified_delay);
        assert_eq!(config.keep_alive_interval, config2.keep_alive_interval);
        assert_eq!(
            config.clash_for_android.append_system_dns,
            config2.clash_for_android.append_system_dns
        );
        assert_eq!(config.sniffer.enable, config2.sniffer.enable);
        assert_eq!(config.proxy_groups.len(), config2.proxy_groups.len());
        assert_eq!(config.proxy_groups[0].name, config2.proxy_groups[0].name);
        assert_eq!(config.rules.len(), config2.rules.len());
    }

    #[test]
    fn test_file_roundtrip() {
        let config = MihomoConfig::default_config();
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let file_path = temp_dir.path().join("test_config.yaml");

        // 由于write_to_file使用固定目录，我们需要修改测试方式
        // 我们直接测试序列化和文件写入
        let yaml = config.to_yaml().expect("序列化YAML失败");
        fs::write(&file_path, &yaml).expect("写入文件失败");

        let content = fs::read_to_string(&file_path).expect("读取文件失败");
        let config2 = MihomoConfig::from_yaml(&content).expect("反序列化YAML失败");

        assert_eq!(config.port, config2.port);
        assert_eq!(config.socks_port, config2.socks_port);
        assert_eq!(config.allow_lan, config2.allow_lan);
        assert_eq!(config.mode, config2.mode);
        assert_eq!(config.log_level, config2.log_level);
        assert_eq!(config.external_controller, config2.external_controller);
        assert_eq!(config.unified_delay, config2.unified_delay);
        assert_eq!(config.keep_alive_interval, config2.keep_alive_interval);
        assert_eq!(
            config.clash_for_android.append_system_dns,
            config2.clash_for_android.append_system_dns
        );
        assert_eq!(config.sniffer.enable, config2.sniffer.enable);
        assert_eq!(config.proxy_groups.len(), config2.proxy_groups.len());
        assert_eq!(config.proxy_groups[0].name, config2.proxy_groups[0].name);
        assert_eq!(config.rules.len(), config2.rules.len());
    }

    #[test]
    fn test_invalid_yaml() {
        let invalid_yaml = "invalid: yaml: content: [";
        let result = MihomoConfig::from_yaml(invalid_yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("解析YAML失败"));
    }

    // #[test]
    // fn test_insert_sub() {
    //     let mut config = MihomoConfig::default_config();
    //     assert!(config.proxy_providers.is_none());

    //     config.insert_sub("https://example.com/sub".to_string());
    //     assert!(config.proxy_providers.is_some());

    //     let providers = config.proxy_providers.as_ref().unwrap();
    //     assert_eq!(providers.len(), 1);

    //     let hash = format!("{:x}", md5::compute("https://example.com/sub"));
    //     assert!(providers.contains_key(&hash));

    //     let provider = providers.get(&hash).unwrap();
    //     assert_eq!(provider.provider_type, "http");
    //     assert_eq!(provider.url, "https://example.com/sub");
    //     assert_eq!(provider.interval, 3600);
    // }

    // #[test]
    // fn test_insert_sub_multiple() {
    //     let mut config = MihomoConfig::default_config();

    //     config.insert_sub("https://example.com/sub1".to_string());
    //     config.insert_sub("https://example.com/sub2".to_string());

    //     let providers = config.proxy_providers.as_ref().unwrap();
    //     assert_eq!(providers.len(), 2);

    //     let hash1 = format!("{:x}", md5::compute("https://example.com/sub1"));
    //     let hash2 = format!("{:x}", md5::compute("https://example.com/sub2"));

    //     assert!(providers.contains_key(&hash1));
    //     assert!(providers.contains_key(&hash2));
    // }
    #[test]
    fn test_write_to_path_and_read_from_path() {
        let config = MihomoConfig::default_config();
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let file_path = temp_dir.path().join("test_config.yaml");

        // 写入到指定路径
        config.write_to_path(&file_path).expect("写入文件失败");

        // 验证文件存在
        assert!(file_path.exists(), "配置文件应该存在");

        // 从指定路径读取
        let config2 = MihomoConfig::read_from_path(&file_path).expect("读取文件失败");

        // 验证配置内容
        assert_eq!(config.port, config2.port);
        assert_eq!(config.socks_port, config2.socks_port);
        assert_eq!(config.allow_lan, config2.allow_lan);
        assert_eq!(config.mode, config2.mode);
        assert_eq!(config.log_level, config2.log_level);
        assert_eq!(config.external_controller, config2.external_controller);
        assert_eq!(config.unified_delay, config2.unified_delay);
        assert_eq!(config.keep_alive_interval, config2.keep_alive_interval);
        assert_eq!(
            config.clash_for_android.append_system_dns,
            config2.clash_for_android.append_system_dns
        );
        assert_eq!(config.sniffer.enable, config2.sniffer.enable);
        assert_eq!(config.proxy_groups.len(), config2.proxy_groups.len());
        assert_eq!(config.proxy_groups[0].name, config2.proxy_groups[0].name);
        assert_eq!(config.rules.len(), config2.rules.len());
    }

    #[test]
    fn test_read_from_path_not_found() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let file_path = temp_dir.path().join("nonexistent.yaml");

        // 尝试读取不存在的文件
        let result = MihomoConfig::read_from_path(&file_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("配置文件不存在"));
    }

    #[test]
    fn test_start_mihomo_config_not_found() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let config_dir: std::path::PathBuf = temp_dir.path().to_path_buf();
        let config_path = config_dir.join("mihomo-config.yaml");

        let mut mihomo = Mihomo::new("mihomo".to_string());

        // 配置文件不存在时应返回错误
        let result = mihomo.start_mihomo();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("配置文件不存在"));
    }

    #[tokio::test]
    async fn test_start_mihomo_success() {
        let temp_dir = TempDir::new().expect("创建临时目录失败");
        let config_path = temp_dir.path().join("config.yaml");

        let mut mihomo = Mihomo::new("mihomo".to_string());
        mihomo.config_path = config_path;

        let filename = mihomo
            .config
            .insert_sub(
                "https://api0.bigmelook.com/BigME/Subscription/api/v1/client/subscribe?token=09bc6f14b110c83776cf13c91f701e44"
                    .to_string(),
                &mihomo.config_path,
            )
            .expect("插入订阅失败");
        mihomo.config.proxy_groups[0].use_list.push(filename);

        let result = mihomo.start_mihomo();
        assert!(result.is_ok(), "启动 mihomo 应该成功: {:?}", result.err());
        eprintln!("开始测试喵");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        match mihomo.test_proxy_delay().await {
            Ok(a) => println!("{}", a),
            Err(e) => println!("跳过延迟测试: {}", e),
        }
        eprintln!("完成测试喵");
        mihomo.stop_mihomo();
    }

    // #[test]
    // fn test_enable_proxy() {
    //     system_proxy::enable_proxy("127.0.0.1.7890").ok();
    // }

    // async fn test_speed() {
    //     let mihomo = Mihomo::new("mihomo-windows-amd64.exe".to_string());
    // }
}
