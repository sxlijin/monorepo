use std::{
    path::Path,
    time::Duration,
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::Value;
use tokio::{
    process::{Child, Command},
    sync::broadcast,
    time,
};

use crate::tasks::db::{ReloadEvent, Task, TaskDb, TaskEntry, TaskSelection};

pub struct TaskScheduler {
    selection: TaskSelection,
    task_db: TaskDb,
    watch_root: std::path::PathBuf,
    fs_rx: broadcast::Receiver<String>,
    reload_rx: broadcast::Receiver<ReloadEvent>,
    current_task: Option<ScheduledTask>,
    running: Option<RunningTask>,
    last_seen_version: u64,
}

#[derive(Clone)]
struct ScheduledTask {
    task: Task,
    hash: u64,
    globset: GlobSet,
}

struct RunningTask {
    child: Child,
    persistent: bool,
}

impl TaskScheduler {
    pub fn new(
        selection: TaskSelection,
        task_db: TaskDb,
        watch_root: std::path::PathBuf,
        fs_rx: broadcast::Receiver<String>,
    ) -> Self {
        let reload_rx = task_db.subscribe();
        TaskScheduler {
            selection,
            task_db,
            watch_root,
            fs_rx,
            reload_rx,
            current_task: None,
            running: None,
            last_seen_version: 0,
        }
    }

    pub fn spawn(self) {
        tokio::spawn(async move {
            let mut scheduler = self;
            scheduler.init_from_db().await;
            scheduler.run().await;
        });
    }

    async fn init_from_db(&mut self) {
        self.load_current_task().await;
        if let Some(task) = self.current_task.as_ref() {
            self.start_task(&format!("initial start of '{}'", self.selection.name), task.hash)
                .await;
        } else {
            eprintln!(
                "error- failed to resolve task: task '{}' not found in {}",
                self.selection.name,
                self.task_db.tasks_path().display()
            );
        }
    }

    async fn run(&mut self) {
        let mut tick = time::interval(Duration::from_millis(250));
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    self.check_running_exit().await;
                }
                Ok(payload) = self.fs_rx.recv() => {
                    self.handle_fs_event(&payload).await;
                }
                Ok(event) = self.reload_rx.recv() => {
                    self.handle_reload_event(event).await;
                }
                else => break,
            }
        }
    }

    async fn load_current_task(&mut self) {
        let snapshot = self.task_db.snapshot();
        self.last_seen_version = snapshot.version;
        let maybe_entry = snapshot.tasks.get(&self.selection.name);
        self.current_task =
            maybe_entry.and_then(|entry| build_scheduled_task(entry.clone()).ok());
        if maybe_entry.is_none() {
            if let Some(mut running) = self.running.take() {
                self.stop_running(&mut running).await;
            }
            eprintln!(
                "error- failed to resolve task: task '{}' not found in {}",
                self.selection.name,
                self.task_db.tasks_path().display()
            );
        }
    }

    async fn handle_reload_event(&mut self, event: ReloadEvent) {
        match event {
            ReloadEvent::ReloadFailed { message } => {
                eprintln!("{message}");
                return;
            }
            ReloadEvent::Reloaded { version, .. } => {
                if version <= self.last_seen_version {
                    return;
                }
                self.last_seen_version = version;
                let before_hash = self.current_task.as_ref().map(|t| t.hash);
                self.load_current_task().await;
                let after_hash = self.current_task.as_ref().map(|t| t.hash);

                if self.running.is_some()
                    && before_hash.is_some()
                    && after_hash.is_some()
                    && before_hash != after_hash
                {
                    self.restart_persistent_on_change().await;
                } else if self.running.is_none() {
                    if let Some(task) = self.current_task.clone() {
                        if task.task.persistent {
                            self.start_task("start persistent after reload", task.hash)
                                .await;
                        }
                    }
                }
            }
        }
    }

    async fn handle_fs_event(&mut self, payload: &str) {
        let Some(task) = self.current_task.clone() else {
            return;
        };

        if !paths_match_task(payload, &self.watch_root, &self.selection.dir, &task.globset) {
            return;
        }

        if task.task.persistent {
            if self.running.is_none() {
                self.start_task("start persistent task", task.hash).await;
            }
        } else {
            self.start_task("rerun non-persistent task", task.hash).await;
        }
    }

    async fn start_task(&mut self, reason: &str, expected_hash: u64) {
        let Some(task) = self.current_task.clone() else {
            return;
        };
        if task.hash != expected_hash {
            return;
        }

        if let Some(mut running) = self.running.take() {
            if running.persistent && task.task.persistent {
                self.running = Some(running);
                return;
            }
            self.stop_running(&mut running).await;
        }

        match spawn_process(&task.task, &self.selection.dir) {
            Ok(child) => {
                println!("task '{}' started ({reason})", self.selection.name);
                self.running = Some(RunningTask {
                    child,
                    persistent: task.task.persistent,
                });
            }
            Err(err) => {
                eprintln!("error starting task '{}': {err}", self.selection.name);
            }
        }
    }

    async fn restart_persistent_on_change(&mut self) {
        if let Some(mut running) = self.running.take() {
            self.stop_running(&mut running).await;
        }
        if let Some(task) = self.current_task.clone() {
            if task.task.persistent {
                self.start_task("restart after config change", task.hash)
                    .await;
            }
        }
    }

    async fn stop_running(&mut self, running: &mut RunningTask) {
        if let Err(err) = running.child.kill().await {
            eprintln!("failed to stop task '{}': {err}", self.selection.name);
        }
        let _ = running.child.wait().await;
    }

    async fn check_running_exit(&mut self) {
        let mut needs_restart = None;
        if let Some(running) = self.running.as_mut() {
            match running.child.try_wait() {
                Ok(Some(_status)) => {
                    needs_restart = Some(running.persistent);
                    self.running = None;
                }
                Ok(None) => {}
                Err(err) => eprintln!("error waiting for task '{}': {err}", self.selection.name),
            }
        }

        if let Some(true) = needs_restart {
            if let Some(task) = self.current_task.clone() {
                self.start_task("restart persistent task after exit", task.hash)
                    .await;
            }
        }
    }
}

fn build_scheduled_task(entry: TaskEntry) -> Result<ScheduledTask, globset::Error> {
    let globset = build_globset(&entry.task.watch)?;
    Ok(ScheduledTask {
        task: entry.task,
        hash: entry.hash,
        globset,
    })
}

fn build_globset(patterns: &[String]) -> Result<GlobSet, globset::Error> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    builder.build()
}

fn spawn_process(task: &Task, dir: &Path) -> tokio::io::Result<Child> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&task.cmd).current_dir(dir);
    cmd.spawn()
}

fn paths_match_task(
    payload: &str,
    watch_root: &Path,
    task_dir: &Path,
    globset: &GlobSet,
) -> bool {
    let Some(paths) = extract_fs_paths(payload) else {
        return false;
    };

    for path_str in paths {
        let full_path = watch_root.join(&path_str);
        if let Ok(rel_to_task) = full_path.strip_prefix(task_dir) {
            let rel = rel_to_task.to_string_lossy();
            if globset.is_match(rel.as_ref()) {
                return true;
            }
        }
    }

    false
}

fn extract_fs_paths(payload: &str) -> Option<Vec<String>> {
    let value: Value = serde_json::from_str(payload).ok()?;
    if value.get("type").and_then(|t| t.as_str()) != Some("fs-event") {
        return None;
    }

    let paths = value.get("paths")?.as_array()?;
    let mut results = Vec::new();
    for p in paths {
        if let Some(s) = p.as_str() {
            results.push(s.to_string());
        }
    }

    Some(results)
}
