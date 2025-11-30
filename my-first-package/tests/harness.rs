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

    pub fn from_testdata(test_name: &str) -> Result<Self> {
        let harness = Self::new(test_name)?;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata").join(test_name);
        copy_dir_all(&root, &harness.dir)?;
        Ok(harness)
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

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    if !src.exists() {
        anyhow::bail!("testdata path not found: {}", src.display());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest_path)?;
        } else {
            fs::create_dir_all(dest_path.parent().unwrap())?;
            fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
