use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn render_url_input(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .title("添加订阅 (Enter 确认, Esc 取消)")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let input_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1)])
        .split(inner);

    let input_text = if app.url_input.is_empty() {
        "请输入订阅 URL...".to_string()
    } else {
        format!("{}▌", app.url_input)
    };

    let style = if app.url_input.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let input = Paragraph::new(input_text)
        .style(style)
        .wrap(Wrap { trim: false });

    f.render_widget(input, input_layout[0]);
}

pub fn render_provider_select(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 40, f.area());

    // 清除背景
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("选择代理商 (Enter 确认, Esc 取消)")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // 构建代理商列表
    let items: Vec<String> = app
        .mihomo
        .config
        .proxy_providers
        .as_ref()
        .map(|providers| {
            providers
                .keys() // 获取所有 key
                .enumerate()
                .map(|(i, key)| {
                    let marker = if i == app.select_provider {
                        ">> "
                    } else {
                        "   "
                    };
                    format!("{}{}", marker, key)
                })
                .collect()
        })
        .unwrap_or_default();

    let list_text = items.join("\n");

    let style = Style::default().fg(Color::White);

    let list = Paragraph::new(list_text).style(style);

    f.render_widget(list, inner);
}
