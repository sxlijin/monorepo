use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let root = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if root.join("drums.wav").exists() {
        process_directory(&root)?;
    } else {
        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                process_directory(&entry.path())?;
            }
        }
    }

    Ok(())
}

fn process_directory(dir: &Path) -> io::Result<()> {
    let drums = dir.join("drums.wav");
    if !drums.exists() {
        return Ok(());
    }

    let hi = dir.join("drums-hi.wav");
    let lo = dir.join("drums-lo.wav");

    fs::create_dir_all(dir)?;

    fs::copy(&drums, &hi)?;
    fs::copy(&drums, &lo)?;

    println!(
        "Copied {:?} to {:?} and {:?}",
        drums.file_name().unwrap_or_default(),
        hi.file_name().unwrap_or_default(),
        lo.file_name().unwrap_or_default()
    );

    Ok(())
}
