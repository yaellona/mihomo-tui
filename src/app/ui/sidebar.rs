use crate::log::Logs;
use ratatui::{
    layout::Constraint,
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

fn wrap_text(text: &str, width: usize) -> String {
    text.chars()
        .collect::<Vec<_>>()
        .chunks(width)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render<'a>(logs: &Logs, width: usize) -> Table<'a> {
    // let mut info_lines = Vec::new();

    let rows: Vec<Row> = logs
        .find_logs(None)
        .iter()
        .map(|log| {
            let wrapped = wrap_text(&log.msg, width as usize);
            let line_count = wrapped.lines().count();

            Row::new(vec![
                Cell::from(log.log_type.as_str().to_string()),
                Cell::from(wrapped),
            ])
            .height(line_count as u16) // 设置行高为换行后的行数
        })
        .collect();

    Table::new(rows, [Constraint::Length(5), Constraint::Min(0)])
        .block(Block::default().title("日志").borders(Borders::ALL))
}
