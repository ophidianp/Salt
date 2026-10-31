use serde::Serialize;

/// One progress update, sent to the UI as a `salt://progress` event.
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    /// Stable id so the UI can update one row instead of adding new ones.
    pub task: String,
    pub label: String,
    pub done: u64,
    pub total: u64,
    pub finished: bool,
}

/// Where the engine reports progress and launcher log lines. The Tauri app
/// implements this by emitting events; tests use `NoopReporter`.
pub trait Reporter: Send + Sync {
    fn progress(&self, p: Progress);
    fn log(&self, level: &str, message: &str);
}

pub struct NoopReporter;

impl Reporter for NoopReporter {
    fn progress(&self, _: Progress) {}
    fn log(&self, level: &str, message: &str) {
        tracing::info!("[{level}] {message}");
    }
}

pub fn step(r: &dyn Reporter, task: &str, label: &str, done: u64, total: u64) {
    r.progress(Progress {
        task: task.to_string(),
        label: label.to_string(),
        done,
        total,
        finished: total > 0 && done >= total,
    });
}

pub fn finish(r: &dyn Reporter, task: &str, label: &str) {
    r.progress(Progress {
        task: task.to_string(),
        label: label.to_string(),
        done: 1,
        total: 1,
        finished: true,
    });
}
