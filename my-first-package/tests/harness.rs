use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use anyhow::{Context, Result};
use tempfile::TempDir;

use my_first_package::tasks::db;

pub struct Harness {
    pub _temp_dir: TempDir,
    pub dir: PathBuf,
}

impl Harness {
    pub fn new(test_name: &str) -> Result<Self> {
        let temp_dir = tempfile::Builder::new()
            .prefix(&format!("watch-cli-{test_name}-"))
            .tempdir()
            .context("create temp dir")?;
        let dir = temp_dir.path().to_path_buf();
        Ok(Self { _temp_dir: temp_dir, dir })
    }

    pub fn write_tasks(&self, contents: &str) -> Result<()> {
        fs::write(self.dir.join(db::TASKS_FILE_NAME), contents)
            .context("write tasks.toml")
    }

    pub fn run_cli(&self, args: &str) -> Result<Command> {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("watch"));
        cmd.current_dir(&self.dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.args(args.split_ascii_whitespace());
        Ok(cmd)
    }

    pub fn spawn_watch(&self, args: &str) -> Result<Child> {
        let mut cmd = self.run_cli(args)?;
        cmd.spawn().context("spawn watch")
    }
}
