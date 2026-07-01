#[derive(Debug, Clone, PartialEq)]
pub enum LogType {
    Info,
    Warn,
    Error,
    Debug,
}
impl LogType {
    pub fn as_str(&self) -> String {
        match self {
            LogType::Info => "Info".to_string(),
            LogType::Warn => "Warn".to_string(),
            LogType::Error => "Error".to_string(),
            LogType::Debug => "Debug".to_string(),
        }
    }
}
#[derive(Debug)]
pub struct Logs {
    logs: Vec<Log>,
}

impl Logs {
    pub fn new() -> Self {
        Self { logs: vec![] }
    }
    pub fn add_log(&mut self, log_type: LogType, msg: String) {
        self.logs.push(Log::new(log_type, msg));
    }
    pub fn find_logs(&self, log_type: Option<LogType>) -> Vec<Log> {
        match log_type {
            Some(log_type) => {
                // 按类型过滤
                self.logs
                    .iter()
                    .filter(|log| log.log_type == log_type)
                    .cloned()
                    .collect()
            }
            None => {
                // 返回所有日志
                self.logs.clone()
            }
        }
    }
}
#[derive(Debug, Clone)]
pub struct Log {
    pub log_type: LogType,
    pub msg: String,
}

impl Log {
    fn new(log_type: LogType, msg: String) -> Self {
        Self {
            log_type: log_type,
            msg: msg,
        }
    }
}
