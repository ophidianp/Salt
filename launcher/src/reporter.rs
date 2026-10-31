//! Sends engine progress + launcher log lines to the UI as events, and keeps
//! the last few thousand launcher log lines for the Logs page.

use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;

use parking_lot::Mutex;
use salt_core::progress::{Progress, Reporter};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct LauncherLog {
    pub time: i64,
    pub level: String,
    pub message: String,
}

pub struct AppReporter {
    app: AppHandle,
    buffer: Mutex<VecDeque<LauncherLog>>,
    file: Mutex<Option<std::fs::File>>,
}

const KEEP: usize = 3000;

impl AppReporter {
    pub fn new(app: AppHandle, log_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&log_dir);
        let file = std::fs::File::create(log_dir.join("launcher.log")).ok();
        Self { app, buffer: Mutex::new(VecDeque::new()), file: Mutex::new(file) }
    }

    pub fn info(&self, m: &str) { self.log("info", m) }
    pub fn warn(&self, m: &str) { self.log("warn", m) }
    pub fn error(&self, m: &str) { self.log("error", m) }

    pub fn recent(&self) -> Vec<LauncherLog> {
        self.buffer.lock().iter().cloned().collect()
    }
}

impl Reporter for AppReporter {
    fn progress(&self, p: Progress) {
        let _ = self.app.emit("salt://progress", p);
    }

    fn log(&self, level: &str, message: &str) {
        let entry = LauncherLog { time: chrono::Utc::now().timestamp_millis(), level: level.to_string(), message: message.to_string() };
        if let Some(f) = self.file.lock().as_mut() {
            let _ = writeln!(f, "[{}] [{}] {}", chrono::Local::now().format("%H:%M:%S"), level.to_uppercase(), message);
        }
        {
            let mut b = self.buffer.lock();
            b.push_back(entry.clone());
            while b.len() > KEEP {
                b.pop_front();
            }
        }
        let _ = self.app.emit("salt://log", entry);
    }
}
