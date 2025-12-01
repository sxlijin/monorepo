use std::{env, net::SocketAddr, path::PathBuf};

use my_first_package::{
    run_watch_server,
    tasks::{TASKS_FILE_NAME, TaskSelection},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = ([127, 0, 0, 1], 8080).into();
    let watch_dir = env::current_dir()?;

    let args: Vec<String> = env::args().collect();
    let (task_selections, had_parse_error) = parse_task_selections(&args, &watch_dir);
    if had_parse_error || task_selections.is_empty() {
        eprintln!("error- failed to resolve task");
    }

    run_watch_server(watch_dir, addr, task_selections).await
}

fn parse_task_selections(args: &[String], cwd: &PathBuf) -> (Vec<TaskSelection>, bool) {
    let mut had_error = false;
    let mut selections = Vec::new();
    if args.len() < 2 {
        return (selections, true);
    }

    for spec in args.iter().skip(1) {
        let (path_part, task_part) = match spec.rsplit_once(':') {
            Some((path, task)) => (path, task),
            None => {
                had_error = true;
                ("", spec.as_str())
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

        if path_part.is_empty() && task_part.is_empty() {
            had_error = true;
        }

        if had_error {
            eprintln!(
                "expected '<path>:<task>', got '{}'; using {} for task '{}'",
                spec,
                selection.dir.join(TASKS_FILE_NAME).display(),
                selection.name
            );
        }

        selections.push(selection);
    }

    (selections, had_error)
}
