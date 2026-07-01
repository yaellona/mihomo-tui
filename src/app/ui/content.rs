// use crate::proxy::ProxyNode;
use crate::config::node::Node;
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

pub fn render<'a>(nodes: &Vec<Node>) -> Table<'a> {
    let rows: Vec<Row> = nodes
        .iter()
        .map(|node| {
            Row::new(vec![
                Cell::from(node.name.clone()),
                Cell::from(node.speed.clone()),
            ])
        })
        .collect();

    let header = Row::new(vec!["名称", "速度"])
        .style(Style::default().fg(Color::Yellow))
        .bottom_margin(1);

    Table::new(rows, [Constraint::Min(0), Constraint::Length(5)])
        .header(header)
        .block(Block::default().title("节点列表").borders(Borders::ALL))
        .row_highlight_style(Style::default().bg(Color::LightBlue))
        .highlight_symbol(">> ")
}
