pub mod content;
pub mod footer;
pub mod header;
pub mod sidebar;
use std::io::Split;

use ratatui::{
    Frame,
    layout::{
        Constraint::{self, Length},
        Direction, Layout,
    },
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    let footer = footer::render(
        "q: 退出 | ↑↓: 导航 | Enter: 启动/停止 | u: 添加订阅 | c: 切换代理商 | p: 系统代理({})",
    );
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            //footer区域
            Constraint::Length(1),
        ])
        .split(size);

    let chunks2 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40), // 侧边栏
        ])
        .split(main_chunks[0]);

    let chunks3 = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // 中间表格
        ])
        .split(chunks2[0]);
    let info = header::render(&app);
    let content = content::render(&app.mihomo.current_nodes);
    let sidebar = sidebar::render(&app.logs, chunks2[1].width as usize - 10);

    f.render_widget(footer, main_chunks[1]);
    f.render_widget(sidebar, chunks2[1]);
    f.render_widget(info, chunks3[0]);
    // f.render_widget(&content, chunks3[1]);

    f.render_stateful_widget(
        &content,
        chunks3[1],
        &mut ratatui::widgets::TableState::default().with_selected(Some(app.select_node)),
    );

    // 弹窗渲染
    // match app.popup {
    //     PopupMode::UrlInput => {
    //         components::popup::render_url_input(f, app);
    //     }
    //     PopupMode::AgencySelect => {
    //         components::popup::render_agency_select(f, app);
    //     }
    //     PopupMode::None => {}
    // }
}
