use crate::app::App;
use crate::constants::MIXED_PORT;
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};
pub fn render(app: &App) -> Table<'_> {
    let rows: Vec<Row> = vec![
        Row::new(vec![
            Cell::from("代理".to_string()),
            Cell::from(format!("127.0.0.1:{}", MIXED_PORT)),
        ]),
        Row::new(vec![
            Cell::from("节点".to_string()),
            Cell::from(if app.active_node.is_none() {
                "无".to_string()
            } else {
                app.current_nodes[app.active_node.unwrap()].name.to_string()
            }),
        ]),
        Row::new(vec![
            Cell::from("mihomo内核".to_string()),
            Cell::from(if app.mihomo_running {
                "运行中".to_string()
            } else {
                "已停止".to_string()
            }),
        ]),
        Row::new(vec![
            Cell::from("系统代理".to_string()),
            Cell::from(if app.proxy_running {
                "开启".to_string()
            } else {
                "关闭".to_string()
            }),
        ]),
        Row::new(vec![
            Cell::from("TUN".to_string()),
            Cell::from(if app.tun_enabled {
                "开启".to_string()
            } else {
                "关闭".to_string()
            }),
        ]),
    ];

    // if !app.current_nodes.is_empty() {
    //     info_lines.push(format!("共{}个节点", app.current_nodes.len()));
    // }

    // let info_text = info_lines.join("\n");
    Table::new(rows, [Constraint::Length(20), Constraint::Min(0)])
        .block(Block::default().title("状态信息").borders(Borders::ALL))

    // Paragraph::new(info_text)
    //     .style(Style::default().fg(Color::Cyan))
    //     .block(Block::default().title("状态信息").borders(Borders::ALL))
}
