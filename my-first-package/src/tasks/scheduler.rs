use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TaskKey {
    root: PathBuf,
    name: String,
}

#[derive(Clone)]
struct ScheduledTask {
    task: Task,
    _hash: u64,
    globset: GlobSet,
}

struct RunningTask {
    child: Child,
    persistent: bool,
}

pub struct TaskScheduler {
    selections: Vec<TaskSelection>,
    task_db: TaskDb,
    watch_root: PathBuf,
    fs_rx: broadcast::Receiver<String>,
    reload_rx: broadcast::Receiver<ReloadEvent>,
    scheduled: HashMap<TaskKey, ScheduledTask>,
    running: HashMap<TaskKey, RunningTask>,
    root_versions: HashMap<PathBuf, u64>,
    roots: Vec<PathBuf>,
}

impl TaskScheduler {
    pub fn new(
        selections: Vec<TaskSelection>,
        task_db: TaskDb,
        watch_root: PathBuf,
        fs_rx: broadcast::Receiver<String>,
    ) -> Self {
        let reload_rx = task_db.subscribe();
        let roots = selections.iter().map(|s| s.dir.clone()).collect();
        TaskScheduler {
            selections,
            task_db,
            watch_root,
            fs_rx,
            reload_rx,
            scheduled: HashMap::new(),
            running: HashMap::new(),
            root_versions: HashMap::new(),
            roots,
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
        let selections = self.selections.clone();
        for selection in selections {
            self.load_task(&selection).await;
        }

        // Start persistent tasks immediately.
        let keys_to_start: Vec<TaskKey> = self
            .scheduled
            .iter()
            .filter_map(|(key, task)| task.task.persistent.then_some(key.clone()))
            .collect();
        for key in keys_to_start {
            self.start_task(&key, "initial persistent start").await;
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

    async fn load_task(&mut self, selection: &TaskSelection) {
        let snapshot = match self.task_db.snapshot(&selection.dir) {
            Some(s) => s,
            None => {
                eprintln!(
                    "error- failed to resolve task: task '{}' not found in {}",
                    selection.name,
                    selection.dir.join(super::db::TASKS_FILE_NAME).display()
                );
                return;
            }
        };

        self.root_versions
            .entry(selection.dir.clone())
            .or_insert(snapshot.version);

        let maybe_entry = snapshot.tasks.get(&selection.name);
        match maybe_entry.and_then(|entry| build_scheduled_task(entry.clone()).ok()) {
            Some(task) => {
                let key = TaskKey {
                    root: selection.dir.clone(),
                    name: selection.name.clone(),
                };
                self.scheduled.insert(key, task);
            }
            None => {
                eprintln!(
                    "error- failed to resolve task: task '{}' not found in {}",
                    selection.name,
                    selection.dir.join(super::db::TASKS_FILE_NAME).display()
                );
            }
        }
    }

    async fn handle_reload_event(&mut self, event: ReloadEvent) {
        match event {
            ReloadEvent::ReloadFailed { root, message } => {
                if self.root_tracked(&root) {
                    eprintln!("{message}");
                }
            }
            ReloadEvent::Reloaded {
                root,
                version,
                changes,
            } => {
                if !self.root_tracked(&root) {
                    return;
                }
                let prev_version = self.root_versions.get(&root).cloned().unwrap_or(0);
                if version <= prev_version {
                    return;
                }
                self.root_versions.insert(root.clone(), version);

                // Refresh scheduled entries for this root.
                let selections: Vec<TaskSelection> = self
                    .selections
                    .iter()
                    .filter(|s| s.dir == root)
                    .cloned()
                    .collect();
                for sel in selections.iter() {
                    self.load_task(sel).await;
                }

                // Stop removed tasks.
                for removed_name in changes.removed {
                    let key = TaskKey {
                        root: root.clone(),
                        name: removed_name.clone(),
                    };
                    if let Some(mut running) = self.running.remove(&key) {
                        self.stop_running(&mut running, &removed_name).await;
                    }
                    self.scheduled.remove(&key);
                }

                // Restart persistent tasks whose definitions changed.
                for changed_name in changes.changed {
                    let key = TaskKey {
                        root: root.clone(),
                        name: changed_name.clone(),
                    };
                    if let Some(task) = self.scheduled.get(&key) {
                        if task.task.persistent {
                            self.start_task(&key, "restart after config change")
                                .await;
                        }
                    }
                }
            }
        }
    }

    async fn handle_fs_event(&mut self, payload: &str) {
        let Some(paths) = extract_fs_paths(payload) else {
            return;
        };

        let mut seen_keys = HashSet::new();
        let mut dispatch_keys = Vec::new();
        for path_str in paths {
            let full_path = self.watch_root.join(&path_str);
            for root in self.roots.iter() {
                if let Ok(rel) = full_path.strip_prefix(root) {
                    let rel_str = rel.to_string_lossy();
                    for (key, task) in self.scheduled.iter() {
                        if &key.root != root {
                            continue;
                        }
                        if task.globset.is_match(rel_str.as_ref()) {
                            if seen_keys.insert(key.clone()) {
                                dispatch_keys.push(key.clone());
                            }
                        }
                    }
                }
            }
        }

        for key in dispatch_keys {
            if let Some(task) = self.scheduled.get(&key).cloned() {
                self.dispatch_task_for_event(key.clone(), task).await;
            }
        }
    }

    async fn dispatch_task_for_event(&mut self, key: TaskKey, task: ScheduledTask) {
        if task.task.persistent {
            if self.running.get(&key).is_none() {
                self.start_task(&key, "start persistent task").await;
            }
        } else {
            self.start_task(&key, "rerun non-persistent task").await;
        }
    }

    async fn start_task(&mut self, key: &TaskKey, reason: &str) {
        let Some(task) = self.scheduled.get(key).cloned() else {
            return;
        };

        if let Some(mut running) = self.running.remove(key) {
            if running.persistent && task.task.persistent {
                self.running.insert(key.clone(), running);
                return;
            }
            self.stop_running(&mut running, &key.name).await;
        }

        match spawn_process(&task.task, &key.root) {
            Ok(child) => {
                println!("task '{}:{}' started ({reason})", key.root.display(), key.name);
                self.running.insert(
                    key.clone(),
                    RunningTask {
                        child,
                        persistent: task.task.persistent,
                    },
                );
            }
            Err(err) => {
                eprintln!("error starting task '{}:{}': {err}", key.root.display(), key.name);
            }
        }
    }

    async fn stop_running(&mut self, running: &mut RunningTask, name: &str) {
        if let Err(err) = running.child.kill().await {
            eprintln!("failed to stop task '{}': {err}", name);
        }
        let _ = running.child.wait().await;
    }

    async fn check_running_exit(&mut self) {
        let mut to_restart = Vec::new();
        let mut to_remove = Vec::new();
        for (key, running) in self.running.iter_mut() {
            match running.child.try_wait() {
                Ok(Some(_status)) => {
                    if running.persistent {
                        to_restart.push(key.clone());
                    }
                    to_remove.push(key.clone());
                }
                Ok(None) => {}
                Err(err) => eprintln!(
                    "error waiting for task '{}:{}': {err}",
                    key.root.display(),
                    key.name
                ),
            }
        }

        for key in to_remove {
            self.running.remove(&key);
        }
        for key in to_restart {
            self.start_task(&key, "restart persistent task after exit")
                .await;
        }
    }

    fn root_tracked(&self, root: &Path) -> bool {
        self.roots.iter().any(|r| r == root)
    }
}

fn build_scheduled_task(entry: TaskEntry) -> Result<ScheduledTask, globset::Error> {
    let globset = build_globset(&entry.task.watch)?;
    Ok(ScheduledTask {
        task: entry.task,
        _hash: entry.hash,
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
