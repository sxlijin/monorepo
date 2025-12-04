mod harness;

use std::{
    fs,
    net::TcpListener,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use anyhow::{Result, bail};
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

fn file_contains(path: &std::path::Path, needle: &str) -> bool {
    fs::read_to_string(path)
        .unwrap_or_default()
        .contains(needle)
}

fn wait_for_lines(path: &Path, min_lines: usize, timeout: Duration) -> Result<()> {
    wait_for_condition(timeout, || Ok(line_count(path) >= min_lines))
}

fn wait_for_contains(path: &Path, needle: &str, timeout: Duration) -> Result<()> {
    wait_for_condition(timeout, || Ok(file_contains(path, needle)))
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

#[test]
#[serial]
fn multi_dir_tasks_are_isolated() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::from_testdata("multidir")?;
    let mut child = harness.spawn_watch("foo:taskfoo bar:taskbar")?;

    let foo_log = harness.dir.join("foo").join("foo.log");
    let bar_log = harness.dir.join("bar").join("bar.log");

    fs::write(harness.dir.join("foo").join("a.txt"), "foo change")?;
    wait_for_condition(Duration::from_secs(6), || Ok(foo_log.exists()))?;
    wait_for_condition(Duration::from_secs(6), || Ok(bar_log.exists()))?;

    fs::write(harness.dir.join("bar").join("b.txt"), "bar change")?;
    wait_for_condition(Duration::from_secs(6), || Ok(bar_log.exists()))?;

    let foo_content = fs::read_to_string(&foo_log).unwrap_or_default();
    let bar_content = fs::read_to_string(&bar_log).unwrap_or_default();
    assert!(foo_content.contains("foo"));
    assert!(bar_content.contains("bar1"));

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn multi_dir_persistent_restarts_on_definition_change() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::from_testdata("multidir")?;
    let mut child = harness.spawn_watch("bar:taskbar")?;

    let bar_log = harness.dir.join("bar").join("bar.log");
    wait_for_condition(Duration::from_secs(6), || Ok(bar_log.exists()))?;
    wait_for_condition(Duration::from_secs(6), || {
        Ok(fs::read_to_string(&bar_log)
            .unwrap_or_default()
            .contains("bar1"))
    })?;

    // Change bar task to bar2 and trigger reload.
    fs::write(
        harness.dir.join("bar").join("tasks.toml"),
        r#"
[taskbar]
cmd = "echo bar2 >> bar.log"
watch = ["*.txt"]
persistent = true
"#,
    )?;

    wait_for_condition(Duration::from_secs(8), || {
        Ok(fs::read_to_string(&bar_log)
            .unwrap_or_default()
            .contains("bar2"))
    })?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn on_reload_none_skips_downstream_on_upstream_change() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("on_reload_none")?;
    harness.write_tasks(
        r#"
[up]
cmd = "echo up >> up.log"
watch = ["up.txt"]
persistent = true

[down]
cmd = "echo down >> down.log"
watch = ["down.txt"]
persistent = false
deps = ["//:up?on_reload=none"]
"#,
    )?;

    let mut child = harness.spawn_watch(":down")?;
    let up_log = harness.dir.join("up.log");
    let down_log = harness.dir.join("down.log");

    // Trigger downstream directly -> both should run (up as dep, down direct).
    fs::write(harness.dir.join("down.txt"), "one")?;
    wait_for_condition(Duration::from_secs(6), || Ok(down_log.exists()))?;
    wait_for_condition(Duration::from_secs(6), || Ok(up_log.exists()))?;
    let down_lines = line_count(&down_log);

    // Change upstream file; up should rerun, down should not (on_reload=none).
    fs::write(harness.dir.join("up.txt"), "two")?;
    wait_for_condition(Duration::from_secs(6), || Ok(line_count(&up_log) >= 2))?;
    assert_eq!(line_count(&down_log), down_lines);

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn initial_run_executes_tasks_once() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("initial_run")?;
    harness.write_tasks(
        r#"
[root]
cmd = "echo root >> root.log"
watch = ["*.py"]
persistent = false
deps = [
  ":child"
]

[child]
cmd = "echo child >> child.log"
watch = ["*.py"]
persistent = false
"#,
    )?;

    let mut child = harness.spawn_watch(":root")?;
    let root_log = harness.dir.join("root.log");
    let child_log = harness.dir.join("child.log");

    wait_for_condition(Duration::from_secs(6), || {
        Ok(root_log.exists() && child_log.exists())
    })?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn dependent_skips_when_upstream_fails() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("dependent_skip")?;
    harness.write_tasks(
        r#"
[dep]
cmd = "if [ -f fail ]; then echo dep-fail >> dep.log && exit 1; else echo dep-ok >> dep.log; fi"
watch = ["*.txt"]
persistent = false

[root]
cmd = "echo root >> root.log"
watch = ["*.txt"]
persistent = false
deps = [":dep"]
"#,
    )?;

    let mut child = harness.spawn_watch(":root")?;
    let dep_log = harness.dir.join("dep.log");
    let root_log = harness.dir.join("root.log");

    // Initial run executes dep then root in order.
    wait_for_condition(Duration::from_secs(8), || {
        Ok(dep_log.exists() && root_log.exists())
    })?;
    let dep_initial = line_count(&dep_log);
    let root_initial = line_count(&root_log);
    let dep_content = fs::read_to_string(&dep_log).unwrap_or_default();
    let root_content = fs::read_to_string(&root_log).unwrap_or_default();
    assert!(
        dep_content.contains("dep-ok"),
        "expected dep to run successfully once"
    );
    assert!(root_content.contains("root"), "expected root to run once");
    assert!(
        dep_initial >= 1 && root_initial >= 1,
        "expected initial runs recorded"
    );

    // Cause upstream failure and retrigger both via watched file change.
    fs::write(harness.dir.join("fail"), "x")?;
    fs::write(harness.dir.join("trigger.txt"), "again")?;
    wait_for_condition(Duration::from_secs(8), || {
        Ok(line_count(&dep_log) >= dep_initial + 1)
    })?;

    // Downstream should be skipped because upstream failed; root log count unchanged.
    thread::sleep(Duration::from_millis(300)); // small buffer to allow scheduler cycle
    assert!(
        file_contains(&dep_log, "dep-fail"),
        "expected upstream to record failure"
    );
    assert_eq!(
        line_count(&root_log),
        root_initial,
        "downstream should be skipped when dep fails"
    );

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn cross_root_dependency_runs() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::from_testdata("crossroot")?;
    let guard = harness.spawn_watch_guard("foo:foo")?;

    let foo_log = harness.dir.join("foo").join("foo.log");
    let bar_log = harness.dir.join("bar").join("bar.log");

    fs::write(harness.dir.join("bar").join("bar.txt"), "bar change")?;
    wait_for_lines(&bar_log, 1, Duration::from_secs(8))?;
    wait_for_lines(&foo_log, 1, Duration::from_secs(8))?;

    let foo_content = fs::read_to_string(&foo_log).unwrap_or_default();
    let bar_content = fs::read_to_string(&bar_log).unwrap_or_default();
    assert!(foo_content.contains("foo-run"));
    assert!(bar_content.contains("bar-run"));

    drop(guard);
    Ok(())
}

#[test]
#[serial]
fn cascaded_reruns_trigger_dependents() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("cascaded")?;
    harness.write_tasks(
        r#"
[dep]
cmd = "echo dep >> dep.log"
watch = ["dep.txt"]
persistent = false

[root]
cmd = "echo root >> root.log"
watch = ["root.txt"]
persistent = false
deps = [":dep"]
"#,
    )?;

    let mut child = harness.spawn_watch(":root")?;
    let dep_log = harness.dir.join("dep.log");
    let root_log = harness.dir.join("root.log");
    wait_for_lines(&dep_log, 1, Duration::from_secs(8))?;
    wait_for_lines(&root_log, 1, Duration::from_secs(8))?;
    let dep_initial = line_count(&dep_log);
    let root_initial = line_count(&root_log);

    // Change upstream file; both should rerun.
    fs::write(harness.dir.join("dep.txt"), "again")?;
    wait_for_lines(&dep_log, dep_initial + 1, Duration::from_secs(8))?;
    wait_for_lines(&root_log, root_initial + 1, Duration::from_secs(8))?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn persistent_restarts_on_exit() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("persistent_restart")?;
    harness.write_tasks(
        r#"
[task1]
cmd = "echo tick >> run.log"
watch = ["*.txt"]
persistent = true
"#,
    )?;

    let mut child = harness.spawn_watch(":task1")?;
    let log = harness.dir.join("run.log");
    wait_for_lines(&log, 1, Duration::from_secs(8))?;
    let first = line_count(&log);

    // Kill the process to trigger restart.
    thread::sleep(Duration::from_millis(300));
    fs::write(harness.dir.join("kill.txt"), "stop")?;
    thread::sleep(Duration::from_millis(100));
    // Let the scheduler notice exit and restart the persistent task.
    wait_for_lines(&log, first + 1, Duration::from_secs(8))?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn multiple_selections_run_independently() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("multi_sel")?;
    harness.write_tasks(
        r#"
[one]
cmd = "echo one >> one.log"
watch = ["one.txt"]
persistent = false

[two]
cmd = "echo two >> two.log"
watch = ["two.txt"]
persistent = false
"#,
    )?;

    let mut child = harness.spawn_watch(":one :two")?;
    let one_log = harness.dir.join("one.log");
    let two_log = harness.dir.join("two.log");
    wait_for_lines(&one_log, 1, Duration::from_secs(6))?;
    wait_for_lines(&two_log, 1, Duration::from_secs(6))?;

    fs::write(harness.dir.join("one.txt"), "a")?;
    fs::write(harness.dir.join("two.txt"), "b")?;
    wait_for_lines(&one_log, 2, Duration::from_secs(6))?;
    wait_for_lines(&two_log, 2, Duration::from_secs(6))?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn invalid_dep_keeps_valid_tasks_running() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::from_testdata("invalid_dep")?;
    let mut child = harness.spawn_watch(":good")?;

    let good_log = harness.dir.join("good.log");
    wait_for_lines(&good_log, 1, Duration::from_secs(6))?;
    let before = line_count(&good_log);

    fs::write(harness.dir.join("trigger.txt"), "x")?;
    wait_for_lines(&good_log, before + 1, Duration::from_secs(6))?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn cycle_detection_marks_tasks_invalid() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::from_testdata("cycle")?;
    let mut child = harness.spawn_watch(":a")?;

    let a_log = harness.dir.join("a.log");
    let b_log = harness.dir.join("b.log");
    thread::sleep(Duration::from_millis(500));
    assert!(!a_log.exists());
    assert!(!b_log.exists());

    fs::write(harness.dir.join("a.txt"), "trigger")?;
    thread::sleep(Duration::from_millis(500));
    assert!(!a_log.exists());
    assert!(!b_log.exists());

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn missing_selection_logs_error_and_exits() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("missing_selection")?;
    harness.write_tasks(
        r#"
[task1]
cmd = "echo task1 >> run.log"
watch = ["*.txt"]
persistent = false
"#,
    )?;

    let mut cmd = harness.run_cli(":missing")?;
    let output = cmd.output()?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("task 'missing' not found"),
        "stderr should mention missing task, got: {stderr}"
    );

    Ok(())
}

#[test]
#[serial]
fn debounce_coalesces_quick_changes() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("debounce")?;
    harness.write_tasks(
        r#"
[task1]
cmd = "echo hit >> run.log"
watch = ["*.txt"]
persistent = false
"#,
    )?;

    let mut child = harness.spawn_watch(":task1")?;
    let log = harness.dir.join("run.log");
    wait_for_lines(&log, 1, Duration::from_secs(6))?;
    let before = line_count(&log);

    fs::write(harness.dir.join("file.txt"), "one")?;
    fs::write(harness.dir.join("file.txt"), "two")?;
    fs::write(harness.dir.join("file.txt"), "three")?;
    wait_for_lines(&log, before + 1, Duration::from_secs(6))?;
    thread::sleep(Duration::from_millis(400));
    assert!(
        line_count(&log) <= before + 2,
        "expected debounced reruns, got {} lines",
        line_count(&log)
    );

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn invalid_nonpersistent_to_persistent_edge_is_rejected() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("invalid_edge")?;
    harness.write_tasks(
        r#"
[up]
cmd = "echo up >> up.log"
watch = ["up.txt"]
persistent = true

[down]
cmd = "echo down >> down.log"
watch = ["down.txt"]
persistent = false
deps = [":up"]
"#,
    )?;

    let mut child = harness.spawn_watch(":down")?;
    let down_log = harness.dir.join("down.log");
    let up_log = harness.dir.join("up.log");

    thread::sleep(Duration::from_millis(600));
    assert!(
        !down_log.exists(),
        "non-persistent task should be invalid and not run"
    );
    assert!(up_log.exists(), "persistent dep still starts");

    fs::write(harness.dir.join("down.txt"), "trigger")?;
    thread::sleep(Duration::from_millis(600));
    assert_eq!(line_count(&down_log), 0);

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn duplicate_task_names_across_roots_disambiguate() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::from_testdata("dup_names")?;
    let guard = harness.spawn_watch_guard("foo:common bar:common")?;

    let foo_log = harness.dir.join("foo").join("foo.log");
    let bar_log = harness.dir.join("bar").join("bar.log");
    wait_for_lines(&foo_log, 1, Duration::from_secs(6))?;
    wait_for_lines(&bar_log, 1, Duration::from_secs(6))?;
    let foo_content = fs::read_to_string(&foo_log).unwrap_or_default();
    let bar_content = fs::read_to_string(&bar_log).unwrap_or_default();
    assert!(foo_content.contains("foo-common"));
    assert!(bar_content.contains("bar-common"));

    drop(guard);
    Ok(())
}

#[test]
#[serial]
fn invalid_reload_keeps_old_state_but_skips_new_runs() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("invalid_reload")?;
    harness.write_tasks(
        r#"
[persist]
cmd = "echo persist >> persist.log"
watch = ["persist.txt"]
persistent = true

[leaf]
cmd = "echo leaf >> leaf.log"
watch = ["leaf.txt"]
persistent = false
"#,
    )?;

    let mut child = harness.spawn_watch(":leaf")?;
    let leaf_log = harness.dir.join("leaf.log");
    wait_for_lines(&leaf_log, 1, Duration::from_secs(6))?;
    let before = line_count(&leaf_log);

    // Write an invalid tasks file to trigger reload failure.
    fs::write(
        harness.dir.join("tasks.toml"),
        r#"
[leaf
cmd = "broken"
"#,
    )?;
    thread::sleep(Duration::from_millis(400));

    // Trigger a change; leaf should be skipped while in invalid state.
    fs::write(harness.dir.join("leaf.txt"), "change")?;
    thread::sleep(Duration::from_millis(600));
    assert_eq!(
        line_count(&leaf_log),
        before,
        "non-persistent task should be skipped after invalid reload"
    );

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn task_removal_stops_persistent_and_dependents_skip_until_readded() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("task_removal")?;
    let initial = r#"
[up]
cmd = "echo up >> up.log"
watch = ["up.txt"]
persistent = true

[down]
cmd = "echo down >> down.log"
watch = ["down.txt"]
persistent = false
deps = [":up"]
"#;
    harness.write_tasks(initial)?;

    let mut child = harness.spawn_watch(":down")?;
    let up_log = harness.dir.join("up.log");
    let down_log = harness.dir.join("down.log");
    wait_for_lines(&up_log, 1, Duration::from_secs(8))?;
    wait_for_lines(&down_log, 1, Duration::from_secs(8))?;
    let down_before = line_count(&down_log);

    // Remove upstream task; downstream should be invalid and skipped.
    harness.write_tasks(
        r#"
[down]
cmd = "echo down >> down.log"
watch = ["down.txt"]
persistent = false
deps = [":up"]
"#,
    )?;
    fs::write(harness.dir.join("down.txt"), "trigger")?;
    thread::sleep(Duration::from_millis(800));
    assert_eq!(
        line_count(&down_log),
        down_before,
        "downstream should not rerun without upstream"
    );

    // Re-add upstream; rerun should succeed.
    harness.write_tasks(initial)?;
    fs::write(harness.dir.join("down.txt"), "again")?;
    wait_for_lines(&down_log, down_before + 1, Duration::from_secs(8))?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn persistent_starts_without_fs_events() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("persistent_startup")?;
    harness.write_tasks(
        r#"
[svc]
cmd = "echo svc >> svc.log"
watch = ["*.txt"]
persistent = true
"#,
    )?;

    let mut child = harness.spawn_watch(":svc")?;
    let log = harness.dir.join("svc.log");
    wait_for_lines(&log, 1, Duration::from_secs(6))?;
    // No fs events written; presence of line confirms startup.
    assert!(file_contains(&log, "svc"));

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn parallel_independent_branches_run_concurrently() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::from_testdata("parallel_branches")?;

    let mut child = harness.spawn_watch(":a :b")?;
    let a_log = harness.dir.join("a.log");
    let b_log = harness.dir.join("b.log");
    wait_for_lines(&a_log, 1, Duration::from_secs(6))?;
    wait_for_lines(&b_log, 1, Duration::from_secs(6))?;
    let a_before = line_count(&a_log);
    let b_before = line_count(&b_log);

    let start = Instant::now();
    fs::write(harness.dir.join("a.txt"), "go")?;
    fs::write(harness.dir.join("b.txt"), "go")?;
    wait_for_lines(&a_log, a_before + 2, Duration::from_secs(6))?;
    wait_for_lines(&b_log, b_before + 2, Duration::from_secs(6))?;
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(1100));

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn non_workspace_defaults_to_cwd_with_relative_deps() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("non_workspace")?;
    harness.write_tasks(
        r#"
[child]
cmd = "echo child >> child.log"
watch = ["child.txt"]
persistent = false

[root]
cmd = "echo root >> root.log"
watch = ["root.txt"]
persistent = false
deps = [":child"]
"#,
    )?;

    let mut child = harness.spawn_watch(":root")?;
    let root_log = harness.dir.join("root.log");
    let child_log = harness.dir.join("child.log");
    wait_for_lines(&root_log, 1, Duration::from_secs(6))?;
    wait_for_lines(&child_log, 1, Duration::from_secs(6))?;

    fs::write(harness.dir.join("child.txt"), "touch")?;
    wait_for_lines(&child_log, 2, Duration::from_secs(6))?;
    wait_for_lines(&root_log, 2, Duration::from_secs(6))?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn mixed_chain_fixture_respects_on_reload_none_and_persistents() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::from_testdata("mixed_chain")?;
    let guard = harness.spawn_watch_guard(":p2 :c")?;

    let p1_log = harness.dir.join("p1.log");
    let p2_log = harness.dir.join("p2.log");
    let a_log = harness.dir.join("a.log");
    let b_log = harness.dir.join("b.log");
    let c_log = harness.dir.join("c.log");

    wait_for_lines(&p1_log, 1, Duration::from_secs(6))?;
    wait_for_lines(&p2_log, 1, Duration::from_secs(6))?;
    wait_for_lines(&a_log, 1, Duration::from_secs(6))?;
    wait_for_lines(&b_log, 1, Duration::from_secs(6))?;
    wait_for_lines(&c_log, 1, Duration::from_secs(6))?;

    let p2_before = line_count(&p2_log);
    let c_before = line_count(&c_log);

    // Trigger upstream persistent; p2 should not rerun due to on_reload=none.
    fs::write(harness.dir.join("p1.txt"), "x")?;
    wait_for_lines(&p1_log, 2, Duration::from_secs(8))?;
    thread::sleep(Duration::from_millis(400));
    assert_eq!(
        line_count(&p2_log),
        p2_before,
        "p2 should not rerun when p1 changes (on_reload=none)"
    );

    // Trigger non-p chain; c should not rerun because edge b->c is on_reload=none.
    fs::write(harness.dir.join("a.txt"), "a2")?;
    wait_for_lines(&a_log, 2, Duration::from_secs(8))?;
    wait_for_lines(&b_log, 2, Duration::from_secs(8))?;
    thread::sleep(Duration::from_millis(400));
    assert_eq!(
        line_count(&c_log),
        c_before,
        "c should not rerun when upstream b changes with on_reload=none"
    );

    drop(guard);
    Ok(())
}

#[test]
#[serial]
fn reload_race_runs_latest_definition() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("reload_race")?;
    harness.write_tasks(
        r#"
[task1]
cmd = "echo start-v1 >> run.log && sleep 1 && echo end-v1 >> run.log"
watch = ["*.txt"]
persistent = false
"#,
    )?;

    let mut child = harness.spawn_watch(":task1")?;
    let log = harness.dir.join("run.log");
    wait_for_lines(&log, 1, Duration::from_secs(8))?;

    // While first run is in-flight, change definition and trigger again.
    harness.write_tasks(
        r#"
[task1]
cmd = "echo start-v2 >> run.log && echo end-v2 >> run.log"
watch = ["*.txt"]
persistent = false
"#,
    )?;
    fs::write(harness.dir.join("trigger.txt"), "go")?;
    wait_for_contains(&log, "end-v2", Duration::from_secs(8))?;

    let content = fs::read_to_string(&log).unwrap_or_default();
    assert!(
        content.contains("start-v2") && content.contains("end-v2"),
        "latest definition should be applied after reload"
    );

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn glob_scope_filters_outside_changes() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("glob_scope")?;
    harness.write_tasks(
        r#"
[task1]
cmd = "echo match >> run.log"
watch = ["foo/*.txt"]
persistent = false
"#,
    )?;

    let mut child = harness.spawn_watch(":task1")?;
    let run_log = harness.dir.join("run.log");

    wait_for_contains(&run_log, "match", Duration::from_secs(6))?;
    let initial = line_count(&run_log);

    // Touch file outside the glob; should not rerun.
    fs::write(harness.dir.join("bar.txt"), "no match")?;
    thread::sleep(Duration::from_millis(400));
    assert_eq!(line_count(&run_log), initial);

    // Touch matching file; should rerun.
    fs::create_dir_all(harness.dir.join("foo"))?;
    fs::write(harness.dir.join("foo").join("hit.txt"), "yes")?;
    wait_for_lines(&run_log, initial + 1, Duration::from_secs(6))?;

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn deps_run_before_dependents() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("ordering")?;
    harness.write_tasks(
        r#"
[dep]
cmd = "echo dep-$(date +%s%3N) >> dep.log"
watch = ["*.txt"]
persistent = false

[root]
cmd = "echo root-$(date +%s%3N) >> root.log"
watch = ["*.txt"]
persistent = false
deps = [":dep"]
"#,
    )?;

    let mut child = harness.spawn_watch(":root")?;
    let dep_log = harness.dir.join("dep.log");
    let root_log = harness.dir.join("root.log");

    fs::write(harness.dir.join("trigger.txt"), "go")?;
    wait_for_lines(&dep_log, 1, Duration::from_secs(8))?;
    wait_for_lines(&root_log, 1, Duration::from_secs(8))?;

    let dep_ts = fs::read_to_string(&dep_log).unwrap_or_default();
    let root_ts = fs::read_to_string(&root_log).unwrap_or_default();
    let dep_last = dep_ts
        .lines()
        .last()
        .unwrap_or("")
        .trim_start_matches("dep-");
    let root_last = root_ts
        .lines()
        .last()
        .unwrap_or("")
        .trim_start_matches("root-");
    let dep_val: i64 = dep_last.parse().unwrap_or(0);
    let root_val: i64 = root_last.parse().unwrap_or(0);
    assert!(
        dep_val <= root_val,
        "dependent should run after dep ({} <= {})",
        dep_val,
        root_val
    );

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}

#[test]
#[serial]
fn persistent_to_persistent_does_not_restart_downstream() -> Result<()> {
    if !port_available() {
        eprintln!("skipping test: port 8080 unavailable");
        return Ok(());
    }

    let harness = Harness::new("persistent_skip")?;
    harness.write_tasks(
        r#"
[up]
cmd = "echo up >> up.log"
watch = ["up.txt"]
persistent = true

[down]
cmd = "echo down >> down.log"
watch = ["down.txt"]
persistent = true
deps = [":up"]
"#,
    )?;

    let mut child = harness.spawn_watch(":down")?;
    let up_log = harness.dir.join("up.log");
    let down_log = harness.dir.join("down.log");

    wait_for_lines(&up_log, 1, Duration::from_secs(8))?;
    wait_for_lines(&down_log, 1, Duration::from_secs(8))?;
    let down_initial = line_count(&down_log);

    // Trigger upstream change; downstream should stay at same count.
    fs::write(harness.dir.join("up.txt"), "change")?;
    wait_for_lines(&up_log, 2, Duration::from_secs(8))?;
    thread::sleep(Duration::from_millis(500));
    assert_eq!(
        line_count(&down_log),
        down_initial,
        "downstream persistent should not restart from upstream persistent"
    );

    let _ = child.kill();
    let _ = child.wait();
    Ok(())
}
