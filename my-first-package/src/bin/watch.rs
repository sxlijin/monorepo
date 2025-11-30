use std::{env, net::SocketAddr, path::PathBuf};

use my_first_package::{
    run_watch_server,
    tasks::{TaskSelection, TASKS_FILE_NAME},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();
    let watch_dir = env::current_dir()?;

    let args: Vec<String> = env::args().collect();
    let (task_selection, had_parse_error) = parse_task_selection(&args, &watch_dir);
    if had_parse_error {
        eprintln!("error- failed to resolve task");
    }

    run_watch_server(watch_dir, addr, task_selection).await
}

fn parse_task_selection(args: &[String], cwd: &PathBuf) -> (TaskSelection, bool) {
    let mut had_error = false;
    let spec = match args.get(1) {
        Some(s) => s.as_str(),
        None => {
            had_error = true;
            ":".into()
        }
    };

    let (path_part, task_part) = match spec.rsplit_once(':') {
        Some((path, task)) => (path, task),
        None => {
            had_error = true;
            ("", spec)
        }
    };

    let task_name = if task_part.is_empty() {
        had_error = true;
        "task"
    } else {
        task_part
    };

    let task_dir = if path_part.is_empty() {
        cwd.clone()
    } else {
        cwd.join(path_part)
    };

    let selection = TaskSelection {
        dir: task_dir,
        name: task_name.to_string(),
    };

    if had_error {
        eprintln!(
            "expected '<path>:<task>', got '{}'; using {} for tasks.toml and task '{}'",
            spec,
            selection.dir.join(TASKS_FILE_NAME).display(),
            selection.name
        );
    }

    (selection, had_error)
}
