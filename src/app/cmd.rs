use crate::app::msg::Msg;
use crate::command::mihomo;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub fn delay(tx: mpsc::Sender<Msg>) {
    tokio::spawn(async move {
        match mihomo::fetch_delays().await {
            Ok(m) => {
                let _ = tx.send(Msg::Delay(m)).await;
            }
            Err(e) => {
                let _ = tx.send(Msg::Error(e)).await;
            }
        }
    });
}

pub fn nodes(tx: mpsc::Sender<Msg>) {
    tokio::spawn(async move {
        match mihomo::get_nodes().await {
            Ok(n) => {
                let _ = tx.send(Msg::Nodes(n)).await;
            }
            Err(e) => {
                let _ = tx.send(Msg::Error(e)).await;
            }
        }
    });
}

pub fn switch_node(tx: mpsc::Sender<Msg>, name: String) {
    tokio::spawn(async move {
        match mihomo::switch_node(name).await {
            Ok(_) => {
                let _ = tx.send(Msg::SwitchedNode).await;
            }
            Err(e) => {
                let _ = tx.send(Msg::Error(e)).await;
            }
        }
    });
}

pub fn reload(tx: mpsc::Sender<Msg>, path: PathBuf) {
    tokio::spawn(async move {
        match mihomo::reload_config(path).await {
            Ok(_) => {
                let _ = tx.send(Msg::SwitchedProvider).await;
            }
            Err(e) => {
                let _ = tx.send(Msg::Error(e)).await;
            }
        }
    });
}

pub fn check_sub(tx: mpsc::Sender<Msg>, sub_name: String, path: PathBuf) {
    tokio::spawn(async move {
        let _ = mihomo::reload_config(path).await;
        let err = mihomo::find_provider(sub_name.clone()).await.err();
        let _ = tx.send(Msg::SubChecked { sub_name, err }).await;
    });
}
