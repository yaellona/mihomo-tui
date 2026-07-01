mod command;
mod config;
mod log;
#[cfg(test)]
mod test;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
};
use std::time::Duration;

use ratatui::{Terminal, backend::CrosstermBackend};
use std::collections::HashMap;
use std::io;
use tokio::time::sleep;
#[tokio::main]
async fn main() -> Result<(), io::Error> {
    // enable_raw_mode()?;
    // let mut stdout = io::stdout();
    // execute!(stdout, EnterAlternateScreen)?;
    // let backend = CrosstermBackend::new(stdout);
    // let terminal = Terminal::new(backend)?;
    let mut mihomo = command::mihomo::Mihomo::new("mihomo-windows-amd64.exe".to_string());
    // mihomo.write_config();
    mihomo.start_mihomo();
    tokio::time::sleep(Duration::from_secs(5)).await;
    // let path = mihomo.config_path.clone();
    match mihomo.update_node().await {
        Ok(_) => println!("{:?}", mihomo.current_node),
        Err(e) => println!("获取节点失败: {}", e),
    }
    match mihomo.test_proxy_delay().await {
        Ok(_) => println!("{:?}", mihomo.current_node),
        Err(e) => println!("测速失败: {}", e),
    }
    loop {}

    Ok(())
}
