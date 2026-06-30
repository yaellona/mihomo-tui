#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;
pub fn enable_proxy(proxy_addr: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let internet_settings = hkcu.open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_WRITE,
        )?;
        internet_settings.set_value("ProxyEnable", &1u32)?;
        internet_settings.set_value("ProxyServer", &proxy_addr)?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "System proxy configuration is not supported on Linux",
        ))
    }
}
pub fn disable_proxy() -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let internet_settings = hkcu.open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_WRITE,
        )?;
        internet_settings.set_value("ProxyEnable", &0u32)?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "System proxy configuration is not supported on Linux",
        ))
    }
}

pub fn get_proxy_status() -> std::io::Result<(u32, String)> {
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let internet_settings = hkcu.open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_READ,
        )?;

        let enable: u32 = internet_settings.get_value("ProxyEnable")?;
        let server: String = internet_settings
            .get_value("ProxyServer")
            .unwrap_or_default();
        Ok((enable, server))
    }

    #[cfg(target_os = "linux")]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "System proxy configuration is not supported on Linux",
        ))
    }
}
