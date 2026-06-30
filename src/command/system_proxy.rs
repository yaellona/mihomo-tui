use winreg::RegKey;
use winreg::enums::*;

pub fn enable_proxy(proxy_addr: &str) -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        KEY_WRITE,
    )?;
    internet_settings.set_value("ProxyEnable", &1u32)?;
    internet_settings.set_value("ProxyServer", &proxy_addr)?;

    Ok(())
}

pub fn disable_proxy() -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let internet_settings = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
        KEY_WRITE,
    )?;
    internet_settings.set_value("ProxyEnable", &0u32)?;
    Ok(())
}

pub fn get_proxy_status() -> std::io::Result<(u32, String)> {
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
