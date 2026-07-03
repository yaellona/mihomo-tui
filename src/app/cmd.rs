use crate::app::App;
use crate::app::PopupMode;
use crate::command::mihomo;
use crate::config::node::Node;
use crate::log::LogType;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub type AsyncTask = Box<dyn FnOnce(&mut App) + Send>;

pub fn delay(tx: mpsc::Sender<AsyncTask>) {
    tokio::spawn(async move {
        let result = mihomo::fetch_delays().await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(map) => {
                    for node in &mut app.current_nodes {
                        if let Some(&d) = map.get(&node.name) {
                            node.speed = format!("{d}ms");
                        } else {
                            node.speed = "-".to_string();
                        }
                    }
                    app.logs.add_log(LogType::Info, "测速完成".to_string());
                    app.is_test_delay = false;
                }
                Err(e) => {
                    app.is_test_delay = false;
                    app.logs.add_log(LogType::Error, e);
                }
            }))
            .await;
    });
}

pub fn nodes(tx: mpsc::Sender<AsyncTask>) {
    tokio::spawn(async move {
        let result = mihomo::get_proxy().await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(proxy) => {
                    app.current_nodes = vec![];
                    app.select_node = 0;
                    for (index, node) in proxy.all.into_iter().enumerate() {
                        if node == proxy.now {
                            app.active_node = Some(index);
                            app.select_node = index;
                        }
                        app.current_nodes.push(Node::new(node));
                    }
                    app.logs.add_log(LogType::Info, "更新代理信息".to_string());
                }
                Err(e) => {
                    app.logs.add_log(LogType::Error, e);
                }
            }))
            .await;
    });
}

pub fn switch_node(tx: mpsc::Sender<AsyncTask>, name: String) {
    tokio::spawn(async move {
        let result = mihomo::switch_node(name).await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(_) => {
                    app.logs.add_log(LogType::Info, "切换节点".to_string());
                }
                Err(e) => {
                    app.logs.add_log(LogType::Error, e);
                }
            }))
            .await;
    });
}

pub fn reload(tx: mpsc::Sender<AsyncTask>, path: PathBuf) {
    tokio::spawn(async move {
        let result = mihomo::reload_config(path).await;
        let _ = tx
            .send(Box::new(move |app| match result {
                Ok(_) => {
                    app.popup_mode = PopupMode::None;
                    app.logs
                        .add_log(LogType::Info, "切换代理商成功".to_string());
                    let tx = app.async_tx.clone();
                    nodes(tx);
                }
                Err(e) => {
                    app.logs.add_log(LogType::Error, e);
                }
            }))
            .await;
    });
}

pub fn check_sub(tx: mpsc::Sender<AsyncTask>, sub_name: String, path: PathBuf) {
    tokio::spawn(async move {
        let _ = mihomo::reload_config(path).await;
        let err = mihomo::find_provider(sub_name.clone()).await.err();
        let _ = tx
            .send(Box::new(move |app| match err {
                None => {
                    app.logs
                        .add_log(LogType::Info, "订阅添加成功".to_string());
                    let tx = app.async_tx.clone();
                    nodes(tx);
                }
                Some(e) => {
                    if let Err(re) = app.mihomo.rollback_sub(&sub_name) {
                        app.logs
                            .add_log(LogType::Error, format!("回滚失败: {re}"));
                    }
                    let path = app.mihomo.config_path.clone();
                    tokio::spawn(async move {
                        let _ = mihomo::reload_config(path).await;
                    });
                    app.logs
                        .add_log(LogType::Error, format!("订阅失败已回滚: {e}"));
                }
            }))
            .await;
    });
}
