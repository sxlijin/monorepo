use std::{
    collections::HashMap,
    fs,
    hash::{Hash, Hasher},
    io,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::SystemTime,
};

use serde::Deserialize;
use tokio::sync::broadcast;
use toml::de::Error as TomlDeError;

pub const TASKS_FILE_NAME: &str = "tasks.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub cmd: String,
    pub watch: Vec<String>,
    pub persistent: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskSelection {
    pub dir: PathBuf,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct TasksMetadata {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub content_hash: u64,
}

#[derive(Clone, Debug)]
pub struct TasksFile {
    pub tasks: HashMap<String, Task>,
    pub metadata: TasksMetadata,
}

impl TasksFile {
    pub fn task(&self, name: &str) -> Result<&Task, TaskConfigError> {
        self.tasks
            .get(name)
            .ok_or_else(|| TaskConfigError::MissingTask {
                name: name.to_string(),
                path: self.metadata.path.clone(),
            })
    }
}

#[derive(Debug)]
pub enum TaskConfigError {
    MissingFile { path: PathBuf },
    ReadFailed { path: PathBuf, source: io::Error },
    ParseFailed { path: PathBuf, source: TomlDeError },
    MissingTask { name: String, path: PathBuf },
    InvalidTask { name: String, path: PathBuf, reason: String },
}

impl std::fmt::Display for TaskConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskConfigError::MissingFile { path } => {
                write!(
                    f,
                    "error- failed to resolve task: tasks.toml not found at {}",
                    path.display()
                )
            }
            TaskConfigError::ReadFailed { path, source } => {
                write!(
                    f,
                    "error- failed to resolve task: could not read {} ({})",
                    path.display(),
                    source
                )
            }
            TaskConfigError::ParseFailed { path, source } => {
                write!(
                    f,
                    "error- malformed tasks.toml at {}: {}",
                    path.display(),
                    source
                )
            }
            TaskConfigError::MissingTask { name, path } => {
                write!(
                    f,
                    "error- failed to resolve task: task '{}' not found in {}",
                    name,
                    path.display()
                )
            }
            TaskConfigError::InvalidTask { name, path, reason } => {
                write!(
                    f,
                    "error- malformed tasks.toml at {} (task '{}'): {}",
                    path.display(),
                    name,
                    reason
                )
            }
        }
    }
}

impl std::error::Error for TaskConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TaskConfigError::ReadFailed { source, .. } => Some(source),
            TaskConfigError::ParseFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawTasksFile {
    #[serde(flatten)]
    tasks: HashMap<String, RawTask>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTask {
    cmd: Option<String>,
    watch: Option<Vec<String>>,
    persistent: Option<bool>,
}

pub fn load_tasks_file(dir: &Path) -> Result<TasksFile, TaskConfigError> {
    let path = dir.join(TASKS_FILE_NAME);
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Err(TaskConfigError::MissingFile { path });
        }
        Err(err) => {
            return Err(TaskConfigError::ReadFailed {
                path,
                source: err,
            });
        }
    };

    let raw: RawTasksFile =
        toml::from_str(&content).map_err(|source| TaskConfigError::ParseFailed {
            path: path.clone(),
            source,
        })?;

    let mut tasks = HashMap::new();
    for (name, raw_task) in raw.tasks {
        let task = validate_task(&path, &name, raw_task)?;
        tasks.insert(name, task);
    }

    let modified = fs::metadata(&path).ok().and_then(|m| m.modified().ok());
    let content_hash = hash_string(&content);
    let metadata = TasksMetadata {
        path,
        modified,
        content_hash,
    };

    Ok(TasksFile { tasks, metadata })
}

fn validate_task(path: &Path, name: &str, raw: RawTask) -> Result<Task, TaskConfigError> {
    let cmd = raw.cmd.ok_or_else(|| TaskConfigError::InvalidTask {
        name: name.to_string(),
        path: path.to_path_buf(),
        reason: "missing field 'cmd'".to_string(),
    })?;

    let watch = raw.watch.ok_or_else(|| TaskConfigError::InvalidTask {
        name: name.to_string(),
        path: path.to_path_buf(),
        reason: "missing field 'watch'".to_string(),
    })?;

    if watch.iter().any(|g| g.trim().is_empty()) {
        return Err(TaskConfigError::InvalidTask {
            name: name.to_string(),
            path: path.to_path_buf(),
            reason: "watch globs must be non-empty strings".to_string(),
        });
    }

    let persistent = raw.persistent.ok_or_else(|| TaskConfigError::InvalidTask {
        name: name.to_string(),
        path: path.to_path_buf(),
        reason: "missing field 'persistent'".to_string(),
    })?;

    Ok(Task {
        cmd,
        watch,
        persistent,
    })
}

fn hash_string(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn hash_task(task: &Task) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    task.cmd.hash(&mut hasher);
    task.watch.hash(&mut hasher);
    task.persistent.hash(&mut hasher);
    hasher.finish()
}

#[derive(Clone, Debug)]
pub(crate) struct TaskEntry {
    pub task: Task,
    pub hash: u64,
}

#[derive(Clone, Debug)]
pub struct TaskSnapshot {
    pub version: u64,
    pub(crate) tasks: HashMap<String, TaskEntry>,
    pub metadata: Option<TasksMetadata>,
}

#[derive(Clone, Debug)]
pub struct TaskChanges {
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum ReloadEvent {
    Reloaded { root: PathBuf, version: u64, changes: TaskChanges },
    ReloadFailed { root: PathBuf, message: String },
}

#[derive(Clone)]
pub struct TaskDb {
    inner: Arc<RwLock<HashMap<PathBuf, RootState>>>,
    reload_tx: broadcast::Sender<ReloadEvent>,
}

#[derive(Clone, Debug)]
struct RootState {
    version: u64,
    tasks: HashMap<String, TaskEntry>,
    metadata: Option<TasksMetadata>,
}

impl TaskDb {
    pub fn new() -> Self {
        let (reload_tx, _) = broadcast::channel(32);
        TaskDb {
            inner: Arc::new(RwLock::new(HashMap::new())),
            reload_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ReloadEvent> {
        self.reload_tx.subscribe()
    }

    pub fn add_root(&self, dir: PathBuf) {
        let mut inner = self.inner.write().expect("task db poisoned");
        inner.entry(dir).or_insert_with(|| RootState {
            version: 0,
            tasks: HashMap::new(),
            metadata: None,
        });
    }

    pub fn reload_root(&self, dir: &Path) -> Result<(), TaskConfigError> {
        let dir = dir.to_path_buf();
        self.add_root(dir.clone());
        let tasks_file = match load_tasks_file(&dir) {
            Ok(file) => file,
            Err(err) => {
                let _ = self.reload_tx.send(ReloadEvent::ReloadFailed {
                    root: dir.clone(),
                    message: err.to_string(),
                });
                return Err(err);
            }
        };
        let mut new_tasks = HashMap::new();
        for (name, task) in tasks_file.tasks.iter() {
            new_tasks.insert(
                name.clone(),
                TaskEntry {
                    task: task.clone(),
                    hash: hash_task(task),
                },
            );
        }

        let (version, changes) = {
            let mut inner = self.inner.write().expect("task db poisoned");
            let state = inner.get_mut(&dir).expect("root must exist");
            let changes = diff_tasks(&state.tasks, &new_tasks);
            state.tasks = new_tasks;
            state.metadata = Some(tasks_file.metadata);
            state.version = state.version.saturating_add(1);
            (state.version, changes)
        };

        let _ = self.reload_tx.send(ReloadEvent::Reloaded {
            root: dir,
            version,
            changes,
        });
        Ok(())
    }

    pub fn snapshot(&self, dir: &Path) -> Option<TaskSnapshot> {
        let inner = self.inner.read().expect("task db poisoned");
        inner.get(dir).map(|state| TaskSnapshot {
            version: state.version,
            tasks: state.tasks.clone(),
            metadata: state.metadata.clone(),
        })
    }

    pub fn roots(&self) -> Vec<PathBuf> {
        let inner = self.inner.read().expect("task db poisoned");
        inner.keys().cloned().collect()
    }
}

fn diff_tasks(
    old: &HashMap<String, TaskEntry>,
    new: &HashMap<String, TaskEntry>,
) -> TaskChanges {
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    for (name, new_entry) in new.iter() {
        if !old.get(name).map(|o| o.hash == new_entry.hash).unwrap_or(false) {
            changed.push(name.clone());
        }
    }

    for name in old.keys() {
        if !new.contains_key(name) {
            removed.push(name.clone());
        }
    }

    TaskChanges { changed, removed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn loads_valid_tasks_file() {
        let dir = tempdir().unwrap();
        let tasks_path = dir.path().join(TASKS_FILE_NAME);
        let mut file = File::create(&tasks_path).unwrap();
        writeln!(
            file,
            r#"
[task1]
cmd = "echo one"
watch = ["*.rs"]
persistent = true

[task2]
cmd = "echo two"
watch = ["src/**"]
persistent = false
"#
        )
        .unwrap();
        drop(file);

        let tasks = load_tasks_file(dir.path()).expect("should load");
        assert_eq!(tasks.metadata.path, tasks_path);
        assert!(tasks.metadata.content_hash != 0);
        assert!(tasks.metadata.modified.is_some());

        let task1 = tasks.task("task1").unwrap();
        assert_eq!(task1.cmd, "echo one");
        assert_eq!(task1.watch, vec!["*.rs"]);
        assert!(task1.persistent);

        let task2 = tasks.task("task2").unwrap();
        assert_eq!(task2.cmd, "echo two");
        assert_eq!(task2.watch, vec!["src/**"]);
        assert!(!task2.persistent);
    }

    #[test]
    fn missing_tasks_file_returns_error() {
        let dir = tempdir().unwrap();
        let err = load_tasks_file(dir.path()).unwrap_err();
        matches!(err, TaskConfigError::MissingFile { .. });
        assert!(format!("{err}").contains("error- failed to resolve task"));
    }

    #[test]
    fn malformed_toml_returns_error() {
        let dir = tempdir().unwrap();
        let tasks_path = dir.path().join(TASKS_FILE_NAME);
        fs::write(&tasks_path, "[task1\ncmd = \"echo\"").unwrap();

        let err = load_tasks_file(dir.path()).unwrap_err();
        matches!(err, TaskConfigError::ParseFailed { .. });
        assert!(format!("{err}").contains("error- malformed tasks.toml"));
    }

    #[test]
    fn missing_required_field_returns_error() {
        let dir = tempdir().unwrap();
        let tasks_path = dir.path().join(TASKS_FILE_NAME);
        fs::write(
            &tasks_path,
            r#"
[task1]
watch = ["*.rs"]
persistent = true
"#,
        )
        .unwrap();

        let err = load_tasks_file(dir.path()).unwrap_err();
        matches!(err, TaskConfigError::InvalidTask { .. });
        assert!(format!("{err}").contains("missing field 'cmd'"));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let dir = tempdir().unwrap();
        let tasks_path = dir.path().join(TASKS_FILE_NAME);
        fs::write(
            &tasks_path,
            r#"
[task1]
cmd = "echo"
watch = ["*.rs"]
persistent = true
extra = "nope"
"#,
        )
        .unwrap();

        let err = load_tasks_file(dir.path()).unwrap_err();
        matches!(err, TaskConfigError::ParseFailed { .. });
        assert!(format!("{err}").contains("error- malformed tasks.toml"));
    }

    #[test]
    fn missing_task_lookup_returns_error() {
        let dir = tempdir().unwrap();
        let tasks_path = dir.path().join(TASKS_FILE_NAME);
        fs::write(
            &tasks_path,
            r#"
[task1]
cmd = "echo"
watch = ["*.rs"]
persistent = true
"#,
        )
        .unwrap();

        let tasks = load_tasks_file(dir.path()).unwrap();
        let err = tasks.task("task2").unwrap_err();
        matches!(err, TaskConfigError::MissingTask { .. });
        assert!(format!("{err}").contains("task 'task2' not found"));
    }
}
