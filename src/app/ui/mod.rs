pub mod content;
pub mod footer;
pub mod header;
pub mod popup;
pub mod sidebar;
use crate::app::PopupMode;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::app::App;
impl App {
    pub fn draw(&mut self, f: &mut Frame) {
        let size = f.area();
        let footer_text = format!(
            "q: 退出 | ↑↓: 导航 | Enter: 启动节点 | u: 添加订阅 | c: 切换代理商 | t: 测速 | r: 刷新节点 | p: 系统代理({}) | s: mihomo({})",
            if self.proxy_running { "开" } else { "关" },
            if self.mihomo_running { "运行中" } else { "已停止" }
        );
        let footer = footer::render(&footer_text);

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
        let info = header::render(&self);
        let content = content::render(&self.current_nodes);
        let sidebar = sidebar::render(&self.logs, chunks2[1].width as usize - 10);

        f.render_widget(footer, main_chunks[1]);
        f.render_widget(sidebar, chunks2[1]);
        f.render_widget(info, chunks3[0]);

        f.render_stateful_widget(
            &content,
            chunks3[1],
            &mut ratatui::widgets::TableState::default().with_selected(Some(self.select_node)),
        );

        // 弹窗渲染
        match self.popup_mode {
            PopupMode::UrlInput => {
                popup::render_url_input(f, self);
            }
            PopupMode::AgencySelect => {
                popup::render_provider_select(f, self);
            }
            _ => {}
        }
    }
}
