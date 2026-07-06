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
            Cell::from(format!("127.0.0.1:{}", MIXED_PORT)).style(Color::LightMagenta),
        ]),
        Row::new(vec![
            Cell::from("节点".to_string()),
            if app.active_node.is_none() {
                Cell::from("无".to_string()).style(Color::LightYellow)
            } else {
                Cell::from(app.current_nodes[app.active_node.unwrap()].name.to_string())
                    .style(Color::LightGreen)
            },
        ]),
        Row::new(vec![
            Cell::from("mihomo内核".to_string()),
            if app.mihomo_running {
                Cell::from("运行中".to_string()).style(Color::LightGreen)
            } else {
                Cell::from("已停止".to_string()).style(Color::LightYellow)
            },
        ]),
        Row::new(vec![
            Cell::from("系统代理".to_string()),
            if app.proxy_running {
                Cell::from("开启".to_string()).style(Color::LightGreen)
            } else {
                Cell::from("关闭".to_string()).style(Color::LightYellow)
            },
        ]),
        Row::new(vec![
            Cell::from("TUN".to_string()),
            if app.tun_enabled {
                Cell::from("开启".to_string()).style(Color::LightGreen)
            } else {
                Cell::from("关闭".to_string()).style(Color::LightYellow)
            },
        ]),
    ];

    Table::new(rows, [Constraint::Length(20), Constraint::Min(0)])
        .block(Block::default().title("状态信息").borders(Borders::ALL))
}
