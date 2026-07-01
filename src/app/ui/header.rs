use crate::app::App;
use ratatui::{
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
pub fn render(app: &App) -> Paragraph<'_> {
    let mut info_lines = Vec::new();

    if app.proxy_running {
        info_lines.push(format!(
            "\u{1f7e2} 代理运行中 (127.0.0.1:{}) - {}",
            7890,
            &app.mihomo.current_nodes[app.active_node.unwrap()].name
        ));
    } else {
        info_lines.push("\u{1f534} 代理已停止".to_string());
    }

    if !app.mihomo.current_nodes.is_empty() {
        info_lines.push(format!("共{}个节点", app.mihomo.current_nodes.len()));
    }

    let info_text = info_lines.join(" | ");

    Paragraph::new(info_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().title("状态信息").borders(Borders::ALL))
}
