use crate::app::App;
use crate::constants::MIXED_PORT;
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
pub fn render(app: &App) -> Paragraph<'_> {
    let mut info_lines = Vec::new();

    if !app.active_node.is_none() {
        info_lines.push(format!(
            "\u{1f7e2} 代理运行中 (127.0.0.1:{MIXED_PORT}) - {}",
            &app.current_nodes[app.active_node.unwrap()].name
        ));
    } else {
        info_lines.push("\u{1f534} 代理已停止".to_string());
    }
    if app.mihomo_running {
        info_lines.push("\u{1f7e2} mihomo内核(运行中)".to_string());
    } else {
        info_lines.push("\u{1f534} mihomo内核(已停止)".to_string());
    }
    if app.proxy_running {
        info_lines.push(format!("系统代理（开启）"));
    } else {
        info_lines.push(format!("系统代理（关闭）"));
    }

    if app.tun_enabled {
        info_lines.push("TUN（开启）".to_string());
    } else {
        info_lines.push("TUN（关闭）".to_string());
    }

    if !app.current_nodes.is_empty() {
        info_lines.push(format!("共{}个节点", app.current_nodes.len()));
    }

    let info_text = info_lines.join(" | ");

    Paragraph::new(info_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().title("状态信息").borders(Borders::ALL))
}
