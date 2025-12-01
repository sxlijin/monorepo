use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    time::Duration,
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde_json::Value;
use tokio::task::JoinSet;
use tokio::{
    process::{Child, Command},
    sync::broadcast,
    time,
};

use crate::tasks::db::{OnReload, ReloadEvent, ResolvedDep, Task, TaskDb, TaskKey, TaskSelection};

pub struct TaskScheduler {
    selections: Vec<TaskSelection>,
    task_db: TaskDb,
    watch_root: PathBuf,
    fs_rx: broadcast::Receiver<String>,
    reload_rx: broadcast::Receiver<ReloadEvent>,
    tasks: HashMap<TaskKey, TaskRuntime>,
    dependents: HashMap<TaskKey, Vec<DepEdge>>,
    topo_order: Vec<TaskKey>,
    running: HashMap<TaskKey, RunningTask>,
    root_versions: HashMap<PathBuf, u64>,
    roots: Vec<PathBuf>,
    last_run_rerun: HashMap<TaskKey, u64>,
    first_run: bool,
    task_states: HashMap<TaskKey, TaskStatus>,
}

#[derive(Clone)]
struct TaskRuntime {
    task: Task,
    globset: GlobSet,
    deps: Vec<ResolvedDep>,
    valid: bool,
    persistent: bool,
    rerun_hash: u64,
    _effective_hash: u64,
}

#[derive(Clone)]
struct DepEdge {
    to: TaskKey,
    on_reload: OnReload,
    upstream_persistent: bool,
    downstream_persistent: bool,
}

struct RunningTask {
    child: Child,
    persistent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TaskStatus {
    Idle,
    Running,
    CompletedSuccess,
    CompletedError,
    Skipped,
    Invalid,
}

enum TaskStart {
    Spawned(Child),
    PersistentStarted,
    AlreadyRunningPersistent,
}

const MAX_PARALLELISM: usize = 8;

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
            tasks: HashMap::new(),
            dependents: HashMap::new(),
            topo_order: Vec::new(),
            running: HashMap::new(),
            root_versions: HashMap::new(),
            roots,
            last_run_rerun: HashMap::new(),
            first_run: true,
            task_states: HashMap::new(),
        }
    }

    pub fn spawn(self) {
        tokio::spawn(async move {
            let mut scheduler = self;
            scheduler.rebuild_from_db().await;
            scheduler.run().await;
        });
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

    async fn rebuild_from_db(&mut self) {
        let mut tasks = HashMap::new();
        let mut dependents: HashMap<TaskKey, Vec<DepEdge>> = HashMap::new();
        let mut root_versions = HashMap::new();

        for root in self.task_db.roots() {
            let snapshot = match self.task_db.snapshot(&root) {
                Some(s) => s,
                None => continue,
            };
            root_versions.insert(root.clone(), snapshot.version);
            for (name, entry) in snapshot.tasks.iter() {
                let key = TaskKey {
                    root: root.clone(),
                    name: name.clone(),
                };

                let globset = match build_globset(&entry.task.watch) {
                    Ok(gs) => gs,
                    Err(err) => {
                        eprintln!(
                            "invalid glob for task '{}:{}': {err}",
                            key.root.display(),
                            key.name
                        );
                        continue;
                    }
                };

                let runtime = TaskRuntime {
                    task: entry.task.clone(),
                    globset,
                    deps: entry.deps.clone(),
                    valid: entry.valid,
                    persistent: entry.task.persistent,
                    rerun_hash: entry.rerun_hash.unwrap_or(entry.hash),
                    _effective_hash: entry.effective_hash.unwrap_or(entry.hash),
                };

                for dep in entry.deps.iter() {
                    dependents
                        .entry(dep.target.clone())
                        .or_default()
                        .push(DepEdge {
                            to: key.clone(),
                            on_reload: dep.on_reload.clone(),
                            upstream_persistent: false,
                            downstream_persistent: runtime.persistent,
                        });
                }

                tasks.insert(key, runtime);
            }
        }

        // Fill upstream persistence flags now that tasks map is complete.
        for (from, edges) in dependents.iter_mut() {
            let upstream_persistent = tasks.get(from).map(|t| t.persistent).unwrap_or(false);
            for edge in edges.iter_mut() {
                edge.upstream_persistent = upstream_persistent;
            }
        }

        let allowed = restrict_to_selections(&self.selections, &tasks);
        let tasks: HashMap<TaskKey, TaskRuntime> = tasks
            .into_iter()
            .filter(|(k, _)| allowed.contains(k))
            .collect();
        let dependents: HashMap<TaskKey, Vec<DepEdge>> = dependents
            .into_iter()
            .filter_map(|(k, edges)| {
                if allowed.contains(&k) {
                    let filtered: Vec<DepEdge> = edges
                        .into_iter()
                        .filter(|e| allowed.contains(&e.to))
                        .collect();
                    Some((k, filtered))
                } else {
                    None
                }
            })
            .collect();

        let topo_order = topo_sort(&tasks);

        // Stop running tasks that disappeared.
        let mut removed: Vec<TaskKey> = self
            .running
            .keys()
            .filter(|k| !tasks.contains_key(*k))
            .cloned()
            .collect();
        for key in removed.drain(..) {
            if let Some(mut running) = self.running.remove(&key) {
                self.stop_running(&mut running, &key.name).await;
            }
        }

        self.tasks = tasks;
        self.dependents = dependents;
        self.topo_order = topo_order;
        self.root_versions = root_versions;
        self.task_states.retain(|k, _| self.tasks.contains_key(k));
        for (key, task) in self.tasks.iter() {
            let entry = self
                .task_states
                .entry(key.clone())
                .or_insert(TaskStatus::Idle);
            if !task.valid {
                *entry = TaskStatus::Invalid;
            } else if matches!(entry, TaskStatus::Invalid) {
                *entry = TaskStatus::Idle;
            }
        }

        // Start persistent tasks (initial or definition changes).
        let keys_to_start: Vec<TaskKey> = self
            .tasks
            .iter()
            .filter_map(|(k, t)| t.persistent.then_some(k.clone()))
            .collect();
        for key in keys_to_start {
            let _ = self.start_task(&key, "persistent start").await;
        }

        if self.first_run {
            let run_set: HashSet<TaskKey> = self.tasks.keys().cloned().collect();
            let direct = run_set.clone();
            self.run_tasks(run_set, direct, "initial run").await;
            self.first_run = false;
        }
    }

    async fn handle_reload_event(&mut self, event: ReloadEvent) {
        match event {
            ReloadEvent::ReloadFailed { root, message } => {
                if self.root_tracked(&root) {
                    eprintln!("{message}");
                }
            }
            ReloadEvent::Reloaded { root, version, .. } => {
                if !self.root_tracked(&root) {
                    return;
                }
                let prev_version = self.root_versions.get(&root).cloned().unwrap_or(0);
                if version <= prev_version {
                    return;
                }
                self.rebuild_from_db().await;
            }
        }
    }

    async fn handle_fs_event(&mut self, payload: &str) {
        let Some(paths) = extract_fs_paths(payload) else {
            return;
        };

        let mut direct = HashSet::new();
        for path_str in paths {
            let full_path = self.watch_root.join(&path_str);
            for (key, task) in self.tasks.iter() {
                if let Ok(rel) = full_path.strip_prefix(&key.root) {
                    let rel_str = rel.to_string_lossy();
                    if task.globset.is_match(rel_str.as_ref()) {
                        direct.insert(key.clone());
                    }
                }
            }
        }

        if direct.is_empty() {
            return;
        }

        let mut run_set: HashSet<TaskKey> = HashSet::new();
        let mut direct_map: HashSet<TaskKey> = HashSet::new();
        for key in direct.iter() {
            collect_closure(
                key,
                &self.tasks,
                &self.dependents,
                &mut run_set,
                &mut direct_map,
            );
            direct_map.insert(key.clone());
        }

        self.run_tasks(run_set, direct_map, "fs event").await;
    }

    fn finish_task(
        &mut self,
        key: &TaskKey,
        status: TaskStatus,
        indegree: &mut HashMap<TaskKey, usize>,
        dependents: &HashMap<TaskKey, Vec<TaskKey>>,
        ready: &mut VecDeque<TaskKey>,
        blocked_by_error: &mut HashSet<TaskKey>,
    ) {
        self.task_states.insert(key.clone(), status.clone());
        let propagate_block = matches!(status, TaskStatus::CompletedError | TaskStatus::Invalid)
            || (matches!(status, TaskStatus::Skipped) && blocked_by_error.contains(key));
        if propagate_block {
            if let Some(edges) = dependents.get(key) {
                for dep in edges {
                    blocked_by_error.insert(dep.clone());
                }
            }
        }

        if let Some(edges) = dependents.get(key) {
            for dep in edges {
                if let Some(d) = indegree.get_mut(dep) {
                    if *d > 0 {
                        *d -= 1;
                    }
                    if *d == 0 {
                        ready.push_back(dep.clone());
                    }
                }
            }
        }
    }

    async fn run_tasks(
        &mut self,
        run_set: HashSet<TaskKey>,
        direct: HashSet<TaskKey>,
        reason: &str,
    ) {
        let mut indegree: HashMap<TaskKey, usize> = HashMap::new();
        let mut adj: HashMap<TaskKey, Vec<TaskKey>> = HashMap::new();

        for key in run_set.iter() {
            if let Some(task) = self.tasks.get(key) {
                indegree.entry(key.clone()).or_insert(0);
                for dep in task.deps.iter() {
                    if run_set.contains(&dep.target) {
                        *indegree.entry(key.clone()).or_insert(0) += 1;
                        adj.entry(dep.target.clone()).or_default().push(key.clone());
                    }
                }
            }
        }

        let mut ready: VecDeque<TaskKey> = indegree
            .iter()
            .filter(|&(_, &d)| d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        let mut join_set: JoinSet<(TaskKey, TaskStatus)> = JoinSet::new();
        let mut running = 0usize;
        let mut blocked_by_error: HashSet<TaskKey> = HashSet::new();

        while !ready.is_empty() || running > 0 {
            while running < MAX_PARALLELISM {
                let Some(key) = ready.pop_front() else {
                    break;
                };

                let status = self
                    .task_states
                    .get(&key)
                    .cloned()
                    .unwrap_or(TaskStatus::Idle);
                if matches!(
                    status,
                    TaskStatus::CompletedSuccess
                        | TaskStatus::CompletedError
                        | TaskStatus::Skipped
                        | TaskStatus::Invalid
                ) {
                    self.finish_task(
                        &key,
                        status,
                        &mut indegree,
                        &adj,
                        &mut ready,
                        &mut blocked_by_error,
                    );
                    continue;
                }
                if blocked_by_error.contains(&key) {
                    self.finish_task(
                        &key,
                        TaskStatus::Skipped,
                        &mut indegree,
                        &adj,
                        &mut ready,
                        &mut blocked_by_error,
                    );
                    continue;
                }

                let Some(task) = self.tasks.get(&key).cloned() else {
                    self.finish_task(
                        &key,
                        TaskStatus::CompletedError,
                        &mut indegree,
                        &adj,
                        &mut ready,
                        &mut blocked_by_error,
                    );
                    continue;
                };

                if !task.valid {
                    eprintln!(
                        "skipping invalid task '{}:{}'",
                        key.root.display(),
                        key.name
                    );
                    self.finish_task(
                        &key,
                        TaskStatus::Invalid,
                        &mut indegree,
                        &adj,
                        &mut ready,
                        &mut blocked_by_error,
                    );
                    continue;
                }

                // Enforce persistent→persistent skip on upstream triggers only.
                let is_direct = direct.contains(&key);
                if !is_direct {
                    if let Some(edges) = self.dependents.get(&key) {
                        let mut skip = false;
                        for edge in edges.iter().filter(|e| e.to == key) {
                            if edge.upstream_persistent && edge.downstream_persistent {
                                skip = true;
                                break;
                            }
                        }
                        if skip {
                            self.finish_task(
                                &key,
                                TaskStatus::Skipped,
                                &mut indegree,
                                &adj,
                                &mut ready,
                                &mut blocked_by_error,
                            );
                            continue;
                        }
                    }
                }

                let last_rerun = self.last_run_rerun.get(&key).cloned().unwrap_or(0);
                if !is_direct && last_rerun == task.rerun_hash {
                    self.finish_task(
                        &key,
                        TaskStatus::CompletedSuccess,
                        &mut indegree,
                        &adj,
                        &mut ready,
                        &mut blocked_by_error,
                    );
                    continue;
                }

                if task.persistent {
                    let why = if is_direct {
                        format!("{reason} (direct)")
                    } else {
                        format!("{reason} (dependent)")
                    };
                    match self.start_task(&key, &why).await {
                        Ok(TaskStart::PersistentStarted)
                        | Ok(TaskStart::AlreadyRunningPersistent) => {
                            self.last_run_rerun.insert(key.clone(), task.rerun_hash);
                            self.finish_task(
                                &key,
                                TaskStatus::CompletedSuccess,
                                &mut indegree,
                                &adj,
                                &mut ready,
                                &mut blocked_by_error,
                            );
                        }
                        Ok(TaskStart::Spawned(_)) => {
                            // Non-persistent path should not return Spawned here.
                            self.finish_task(
                                &key,
                                TaskStatus::CompletedError,
                                &mut indegree,
                                &adj,
                                &mut ready,
                                &mut blocked_by_error,
                            );
                        }
                        Err(()) => {
                            self.finish_task(
                                &key,
                                TaskStatus::CompletedError,
                                &mut indegree,
                                &adj,
                                &mut ready,
                                &mut blocked_by_error,
                            );
                        }
                    }
                    continue;
                }

                let why = if is_direct {
                    format!("{reason} (direct)")
                } else {
                    format!("{reason} (dependent)")
                };
                match self.start_task(&key, &why).await {
                    Ok(TaskStart::Spawned(mut child)) => {
                        self.task_states.insert(key.clone(), TaskStatus::Running);
                        let k = key.clone();
                        join_set.spawn(async move {
                            let status = match child.wait().await {
                                Ok(exit) if exit.success() => TaskStatus::CompletedSuccess,
                                Ok(_exit) => TaskStatus::CompletedError,
                                Err(_) => TaskStatus::CompletedError,
                            };
                            (k, status)
                        });
                        running += 1;
                    }
                    Ok(TaskStart::PersistentStarted) | Ok(TaskStart::AlreadyRunningPersistent) => {
                        self.last_run_rerun.insert(key.clone(), task.rerun_hash);
                        self.finish_task(
                            &key,
                            TaskStatus::CompletedSuccess,
                            &mut indegree,
                            &adj,
                            &mut ready,
                            &mut blocked_by_error,
                        );
                    }
                    Err(()) => {
                        self.finish_task(
                            &key,
                            TaskStatus::CompletedError,
                            &mut indegree,
                            &adj,
                            &mut ready,
                            &mut blocked_by_error,
                        );
                    }
                }
            }

            if running == 0 && ready.is_empty() {
                break;
            }

            if let Some(res) = join_set.join_next().await {
                running = running.saturating_sub(1);
                match res {
                    Ok((key, status)) => {
                        if matches!(status, TaskStatus::CompletedSuccess) {
                            if let Some(task) = self.tasks.get(&key) {
                                self.last_run_rerun.insert(key.clone(), task.rerun_hash);
                            }
                        }
                        self.finish_task(
                            &key,
                            status,
                            &mut indegree,
                            &adj,
                            &mut ready,
                            &mut blocked_by_error,
                        );
                    }
                    Err(err) => {
                        eprintln!("error joining task handle: {err}");
                    }
                }
            }
        }
    }

    async fn start_task(&mut self, key: &TaskKey, reason: &str) -> Result<TaskStart, ()> {
        let Some(task) = self.tasks.get(key).cloned() else {
            return Err(());
        };

        if let Some(mut running) = self.running.remove(key) {
            if running.persistent && task.persistent {
                self.running.insert(key.clone(), running);
                self.last_run_rerun.insert(key.clone(), task.rerun_hash);
                return Ok(TaskStart::AlreadyRunningPersistent);
            }
            self.stop_running(&mut running, &key.name).await;
        }

        match spawn_process(&task.task, &key.root) {
            Ok(child) => {
                println!(
                    "task '{}:{}' started ({reason})",
                    key.root.display(),
                    key.name
                );
                if task.persistent {
                    self.running.insert(
                        key.clone(),
                        RunningTask {
                            child,
                            persistent: true,
                        },
                    );
                    self.last_run_rerun.insert(key.clone(), task.rerun_hash);
                    Ok(TaskStart::PersistentStarted)
                } else {
                    Ok(TaskStart::Spawned(child))
                }
            }
            Err(err) => {
                eprintln!(
                    "error starting task '{}:{}': {err}",
                    key.root.display(),
                    key.name
                );
                Err(())
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
            let _ = self
                .start_task(&key, "restart persistent task after exit")
                .await;
        }
    }

    fn root_tracked(&self, root: &Path) -> bool {
        self.roots.iter().any(|r| r == root)
    }
}

fn topo_sort(tasks: &HashMap<TaskKey, TaskRuntime>) -> Vec<TaskKey> {
    let mut indeg: HashMap<TaskKey, usize> = HashMap::new();
    let mut adj: HashMap<TaskKey, Vec<TaskKey>> = HashMap::new();

    for (key, task) in tasks.iter() {
        indeg.entry(key.clone()).or_insert(0);
        for dep in task.deps.iter() {
            adj.entry(dep.target.clone()).or_default().push(key.clone());
            *indeg.entry(key.clone()).or_insert(0) += 1;
        }
    }

    let mut q: VecDeque<TaskKey> = indeg
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(k, _)| k.clone())
        .collect();

    let mut order = Vec::new();
    while let Some(node) = q.pop_front() {
        order.push(node.clone());
        if let Some(edges) = adj.get(&node) {
            for nxt in edges {
                if let Some(e) = indeg.get_mut(nxt) {
                    *e -= 1;
                    if *e == 0 {
                        q.push_back(nxt.clone());
                    }
                }
            }
        }
    }

    if order.len() != indeg.len() {
        // cycle detected; return any order (already marked invalid in TaskDb).
        return indeg.keys().cloned().collect();
    }

    order
}

fn collect_closure(
    start: &TaskKey,
    tasks: &HashMap<TaskKey, TaskRuntime>,
    dependents: &HashMap<TaskKey, Vec<DepEdge>>,
    run_set: &mut HashSet<TaskKey>,
    direct: &mut HashSet<TaskKey>,
) {
    let mut stack = vec![start.clone()];
    while let Some(key) = stack.pop() {
        if !run_set.insert(key.clone()) {
            continue;
        }
        direct.insert(start.clone());

        // include deps (ancestors)
        if let Some(task) = tasks.get(&key) {
            for dep in task.deps.iter() {
                stack.push(dep.target.clone());
            }
        }

        // include dependents (reverse edges)
        if let Some(edges) = dependents.get(&key) {
            for edge in edges {
                // persistent->persistent should not force downstream reloads
                if edge.upstream_persistent && edge.downstream_persistent {
                    continue;
                }
                if edge.on_reload == OnReload::None {
                    continue;
                }
                stack.push(edge.to.clone());
            }
        }
    }
}

fn restrict_to_selections(
    selections: &[TaskSelection],
    tasks: &HashMap<TaskKey, TaskRuntime>,
) -> HashSet<TaskKey> {
    let mut allowed = HashSet::new();
    let mut stack = Vec::new();

    for sel in selections {
        let key = TaskKey {
            root: sel.dir.clone(),
            name: sel.name.clone(),
        };
        if !tasks.contains_key(&key) {
            eprintln!(
                "error- failed to resolve task: task '{}' not found in {}",
                sel.name,
                sel.dir.display()
            );
            continue;
        }
        stack.push(key);
    }

    while let Some(key) = stack.pop() {
        if !allowed.insert(key.clone()) {
            continue;
        }
        if let Some(task) = tasks.get(&key) {
            for dep in task.deps.iter() {
                stack.push(dep.target.clone());
            }
        }
    }

    allowed
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
