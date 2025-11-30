mod harness;

use std::{
    fs,
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use harness::Harness;
use serial_test::serial;

fn port_available() -> bool {
    TcpListener::bind("127.0.0.1:8080").is_ok()
}

fn wait_for_condition<F>(timeout: Duration, mut f: F) -> Result<()>
where
    F: FnMut() -> Result<bool>,
{
    let start = Instant::now();
    loop {
        if f()? {
            return Ok(());
        }
        if start.elapsed() > timeout {
            bail!("condition not met within {:?}", timeout);
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn line_count(path: &std::path::Path) -> usize {
    fs::read_to_string(path)
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

#[test]
#[serial]
fn non_persistent_reruns_on_changes() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("non_persistent")?;
    harness.write_tasks(
        r#"
[task1]
cmd = "echo run >> run.log"
watch = ["*.txt"]
persistent = false
"#,
    )?;

    let mut child = harness.spawn_watch(":task1")?;

    let run_log = harness.dir.join("run.log");
    wait_for_condition(Duration::from_secs(5), || Ok(run_log.exists()))?;
    let initial_lines = line_count(&run_log);

    fs::write(harness.dir.join("file1.txt"), "one")?;
    wait_for_condition(Duration::from_secs(5), || {
        Ok(line_count(&run_log) >= initial_lines + 1)
    })?;

    fs::write(harness.dir.join("file1.txt"), "two")?;
    wait_for_condition(Duration::from_secs(5), || {
        Ok(line_count(&run_log) >= initial_lines + 2)
    })?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn persistent_restarts_on_config_change() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("persistent_reload")?;
    harness.write_tasks(
        r#"
[task1]
cmd = "echo v1 >> run.log"
watch = ["*.txt"]
persistent = true
"#,
    )?;

    let mut child = harness.spawn_watch(":task1")?;

    let run_log = harness.dir.join("run.log");
    wait_for_condition(Duration::from_secs(5), || Ok(run_log.exists()))?;
    wait_for_condition(Duration::from_secs(5), || {
        Ok(fs::read_to_string(&run_log)
            .unwrap_or_default()
            .contains("v1"))
    })?;

    harness.write_tasks(
        r#"
[task1]
cmd = "echo v2 >> run.log"
watch = ["*.txt"]
persistent = true
"#,
    )?;

    wait_for_condition(Duration::from_secs(8), || {
        Ok(fs::read_to_string(&run_log)
            .unwrap_or_default()
            .contains("v2"))
    })?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}
