//! End to end tests. Each one drives the real binary through `sh`, evaluating `icicle env`
//! the way a shell profile does, against a throwaway icicle home.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const ICICLE: &str = env!("CARGO_BIN_EXE_icicle");

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A scratch directory containing an icicle home. Deliberately contains a space to make sure
/// nothing relies on unquoted paths.
struct Sandbox {
    root: PathBuf,
    home: PathBuf,
    work: PathBuf,
}

impl Sandbox {
    fn new() -> Sandbox {
        Sandbox::with_home_name("icicle home")
    }

    fn with_home_name(name: &str) -> Sandbox {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("icicle e2e {}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&root);
        let home = root.join(name);
        let work = root.join("work");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&work).unwrap();
        Sandbox { root, home, work }
    }

    /// A fake toolchain whose `yosys` prints the version it belongs to
    fn add_toolchain(&self, version: &str) {
        let bin = self
            .home
            .join("toolchains")
            .join(version)
            .join("oss-cad-suite/bin");
        fs::create_dir_all(&bin).unwrap();
        let tool = bin.join("yosys");
        fs::write(&tool, format!("#!/bin/sh\necho yosys-{}\n", version)).unwrap();
        fs::set_permissions(&tool, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    }

    fn default_target(&self) -> Option<PathBuf> {
        fs::read_link(self.home.join("aliases/default")).ok()
    }

    /// Run a script in `sh` after the startup line a shell profile would have (`eval "$(icicle env)"`)
    fn sh(&self, script: &str) -> Output {
        let bin_dir = Path::new(ICICLE).parent().unwrap();
        let out = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("{}\n{}", r#"eval "$(icicle env)""#, script))
            .current_dir(&self.work)
            .env_clear()
            .env("PATH", format!("{}:/usr/bin:/bin", bin_dir.display()))
            .env("HOME", &self.root)
            .env("ICICLE_HOME", &self.home)
            .output()
            .unwrap();
        Output::from(out)
    }

    /// Like `sh` but without evaluating `icicle env` first
    fn raw(&self, script: &str) -> Output {
        let bin_dir = Path::new(ICICLE).parent().unwrap();
        Output::from(
            Command::new("/bin/sh")
                .arg("-c")
                .arg(script)
                .current_dir(&self.work)
                .env_clear()
                .env("PATH", format!("{}:/usr/bin:/bin", bin_dir.display()))
                .env("HOME", &self.root)
                .env("ICICLE_HOME", &self.home)
                .output()
                .unwrap(),
        )
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct Output {
    ok: bool,
    stdout: String,
    stderr: String,
}

impl From<std::process::Output> for Output {
    fn from(o: std::process::Output) -> Self {
        Output {
            ok: o.status.success(),
            stdout: String::from_utf8_lossy(&o.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).trim().to_string(),
        }
    }
}

impl Output {
    fn success(self) -> String {
        assert!(self.ok, "command failed: {}", self.stderr);
        self.stdout
    }
}

#[test]
fn use_in_a_fresh_shell_without_a_default() {
    // Regression: the new session link points at a default that doesn't exist yet, and
    // `use` used to fail with "File exists" because it didn't see the dangling link.
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");

    let out = sb
        .sh("icicle use 2024-01-01 && icicle current && yosys")
        .success();
    assert_eq!(
        out,
        "Using toolchain 2024-01-01\n2024-01-01\nyosys-2024-01-01"
    );
}

#[test]
fn use_switches_the_tool_on_the_path() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");

    let out = sb
        .sh("icicle use 2024-01-01 >/dev/null; yosys; icicle use 2024-02-02 >/dev/null; yosys; icicle current")
        .success();
    assert_eq!(out, "yosys-2024-01-01\nyosys-2024-02-02\n2024-02-02");
}

#[test]
fn new_shells_follow_the_default() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");

    sb.sh("icicle default 2024-01-01").success();
    assert_eq!(
        sb.sh("icicle current; yosys").success(),
        "2024-01-01\nyosys-2024-01-01"
    );

    // changing the default moves new shells, and shells that haven't picked a version
    sb.sh("icicle default 2024-02-02").success();
    assert_eq!(
        sb.sh("icicle current; yosys").success(),
        "2024-02-02\nyosys-2024-02-02"
    );
}

#[test]
fn use_does_not_change_the_default_or_other_shells() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");
    sb.sh("icicle default 2024-01-01").success();

    sb.sh("icicle use 2024-02-02").success();
    assert_eq!(sb.sh("icicle current").success(), "2024-01-01");
}

#[test]
fn default_can_be_changed_repeatedly() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");

    for version in ["2024-01-01", "2024-02-02", "2024-01-01"] {
        sb.sh(&format!("icicle default {}", version)).success();
        assert!(sb.default_target().unwrap().ends_with(version));
    }
}

#[test]
fn default_recovers_from_a_dangling_default() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    fs::create_dir_all(sb.home.join("aliases")).unwrap();
    unix_fs::symlink(
        sb.home.join("toolchains/gone"),
        sb.home.join("aliases/default"),
    )
    .unwrap();

    sb.sh("icicle default 2024-01-01").success();
    assert!(sb.default_target().unwrap().ends_with("2024-01-01"));
}

#[test]
fn nested_shell_keeps_one_session_and_use_still_applies() {
    // A re-sourced profile or a child shell runs `icicle env` again with the previous session
    // still in the environment. The tool on the PATH must stay under `icicle use`'s control.
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");

    let out = sb
        .sh(r#"
            icicle use 2024-01-01 >/dev/null
            eval "$(icicle env)"
            echo "$PATH" | tr ':' '\n' | grep -c '/caches/icicle_'
            yosys
            icicle use 2024-02-02 >/dev/null
            yosys
        "#)
        .success();
    // two entries (bin + libexec) for exactly one session, and the nested shell inherited the
    // parent's selection
    assert_eq!(out, "2\nyosys-2024-01-01\nyosys-2024-02-02");
}

#[test]
fn nested_shell_does_not_affect_its_parent() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");

    let out = sb
        .sh(r#"
            icicle use 2024-01-01 >/dev/null
            ( eval "$(icicle env)"; icicle use 2024-02-02 >/dev/null; icicle current )
            icicle current
        "#)
        .success();
    assert_eq!(out, "2024-02-02\n2024-01-01");
}

#[test]
fn use_reads_the_toolchain_file() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    fs::write(sb.work.join(".icicle-toolchain"), "2024-01-01\n").unwrap();

    assert_eq!(
        sb.sh("icicle use >/dev/null; icicle current").success(),
        "2024-01-01"
    );
}

#[test]
fn use_can_select_an_alias() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.sh("icicle default 2024-01-01").success();

    assert_eq!(
        sb.sh("icicle use default >/dev/null; icicle current")
            .success(),
        "2024-01-01"
    );
}

#[test]
fn use_unknown_version_fails_and_keeps_the_current_one() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");

    let out = sb.sh("icicle use 2024-01-01 >/dev/null; icicle use 1999-01-01; icicle current");
    assert!(out.stdout.ends_with("2024-01-01"), "{}", out.stdout);
    assert!(sb
        .sh("icicle use 1999-01-01")
        .stderr
        .contains("Unable to find toolchain"));
}

#[test]
fn use_recreates_a_session_link_that_was_cleaned_up() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");

    let out = sb
        .sh(r#"
            rm "$ICICLE_SHELL_PATH"
            icicle use 2024-01-01 >/dev/null
            icicle current
            yosys
        "#)
        .success();
    assert_eq!(out, "2024-01-01\nyosys-2024-01-01");
}

#[test]
fn current_without_any_toolchain_explains_what_to_do() {
    let sb = Sandbox::new();
    let out = sb.sh("icicle current");
    assert!(!out.ok);
    assert!(out.stderr.contains("icicle use"), "{}", out.stderr);
}

#[test]
fn current_works_when_home_is_behind_a_symlink() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    let link = sb.root.join("linked home");
    unix_fs::symlink(&sb.home, &link).unwrap();

    let out = sb
        .raw(&format!(
            r#"export ICICLE_HOME='{}'; eval "$(icicle env)"; icicle use 2024-01-01 >/dev/null; icicle current"#,
            link.display()
        ))
        .success();
    assert_eq!(out, "2024-01-01");
}

#[test]
fn env_requires_no_existing_configuration() {
    let sb = Sandbox::new();
    let out = Command::new(ICICLE)
        .arg("env")
        .env_clear()
        .env("HOME", &sb.root)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&format!("ICICLE_HOME='{}/.icicle'", sb.root.display())));
    assert!(stdout.contains("export ICICLE_SHELL_PATH="));
}

#[test]
fn env_survives_stale_and_unreadable_caches() {
    let sb = Sandbox::new();
    let caches = sb.home.join("caches");
    fs::create_dir_all(&caches).unwrap();
    unix_fs::symlink("/nonexistent", caches.join("icicle_stale")).unwrap();
    fs::write(caches.join("unrelated-file"), "x").unwrap();

    sb.sh("true").success();
    assert!(caches.join("unrelated-file").exists());
}

#[test]
fn env_removes_only_old_sessions() {
    let sb = Sandbox::new();
    let caches = sb.home.join("caches");
    fs::create_dir_all(&caches).unwrap();
    let old = caches.join("icicle_old");
    let fresh = caches.join("icicle_fresh");
    unix_fs::symlink("/nonexistent", &old).unwrap();
    unix_fs::symlink("/nonexistent", &fresh).unwrap();

    // back date the link itself, not what it points at
    let eight_days_ago = "202001010000";
    assert!(Command::new("touch")
        .args(["-h", "-t", eight_days_ago])
        .arg(&old)
        .status()
        .unwrap()
        .success());

    sb.sh("true").success();
    assert!(
        old.symlink_metadata().is_err(),
        "old session should be removed"
    );
    assert!(
        fresh.symlink_metadata().is_ok(),
        "recent session should be kept"
    );
}

#[test]
fn list_marks_the_default_and_ignores_stray_entries() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-02-02");
    sb.add_toolchain("2024-01-01");
    fs::write(sb.home.join("toolchains/.DS_Store"), "").unwrap();
    fs::create_dir_all(sb.home.join("toolchains/.2024-03-03.partial")).unwrap();

    // no default yet
    assert_eq!(sb.sh("icicle list").success(), "* 2024-01-01\n* 2024-02-02");

    sb.sh("icicle default 2024-02-02").success();
    assert_eq!(
        sb.sh("icicle list").success(),
        "* 2024-01-01\n* 2024-02-02 (default)"
    );
}

#[test]
fn uninstall_removes_the_toolchain_and_a_dangling_default() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");
    sb.sh("icicle default 2024-01-01").success();

    sb.sh("icicle uninstall 2024-02-02").success();
    assert!(!sb.home.join("toolchains/2024-02-02").exists());
    assert!(sb.default_target().is_some(), "unrelated default must stay");

    sb.sh("icicle uninstall 2024-01-01").success();
    assert!(
        sb.default_target().is_none(),
        "dangling default should be removed"
    );
    // and a new shell can still start and pick something
    sb.add_toolchain("2024-03-03");
    assert_eq!(
        sb.sh("icicle use 2024-03-03 >/dev/null; icicle current")
            .success(),
        "2024-03-03"
    );
}

#[test]
fn versions_cannot_escape_the_toolchains_directory() {
    let sb = Sandbox::new();
    let victim = sb.root.join("precious");
    fs::create_dir_all(&victim).unwrap();

    for cmd in [
        "icicle uninstall ../../precious",
        "icicle uninstall ..",
        "icicle default ../../precious",
        "icicle use ../../precious",
        "icicle install ../../precious",
    ] {
        let out = sb.sh(cmd);
        assert!(!out.ok, "{cmd} should fail");
        assert!(
            out.stderr.contains("not a valid toolchain version"),
            "{cmd}: {}",
            out.stderr
        );
    }
    assert!(victim.exists());
    assert!(sb.home.join("toolchains").exists());
}

#[test]
fn paths_with_special_characters_survive_eval() {
    // the home contains both a space and a quote
    let sb = Sandbox::with_home_name("it's home");
    sb.add_toolchain("2024-01-01");

    let out = sb.sh("icicle use 2024-01-01 >/dev/null; yosys").success();
    assert_eq!(out, "yosys-2024-01-01");
}

#[test]
fn commands_explain_a_missing_environment() {
    let sb = Sandbox::new();
    let out = Command::new(ICICLE)
        .arg("list")
        .env_clear()
        .env("HOME", &sb.root)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("ICICLE_HOME"));
}

// ---- version file lookup -------------------------------------------------------------------

#[test]
fn version_file_applies_from_subdirectories() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    fs::write(sb.work.join(".icicle-toolchain"), "2024-01-01\n").unwrap();
    fs::create_dir_all(sb.work.join("src/deeply/nested")).unwrap();

    let out = sb
        .sh("cd src/deeply/nested && icicle use >/dev/null && icicle current")
        .success();
    assert_eq!(out, "2024-01-01");
}

#[test]
fn nearest_version_file_wins_over_one_further_up() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");
    fs::write(sb.work.join(".icicle-toolchain"), "2024-01-01").unwrap();
    fs::create_dir_all(sb.work.join("sub")).unwrap();
    fs::write(sb.work.join("sub/.icicle-toolchain"), "2024-02-02").unwrap();

    let out = sb
        .sh("cd sub && icicle use >/dev/null && icicle current")
        .success();
    assert_eq!(out, "2024-02-02");
}

// ---- exec ----------------------------------------------------------------------------------

#[test]
fn exec_runs_a_command_with_the_chosen_toolchain() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");

    // the shell is on 2024-02-02, exec asks for 2024-01-01 and leaves the shell alone
    let out = sb
        .sh("icicle use 2024-02-02 >/dev/null; icicle exec --using 2024-01-01 yosys; yosys; icicle current")
        .success();
    assert_eq!(out, "yosys-2024-01-01\nyosys-2024-02-02\n2024-02-02");
}

#[test]
fn exec_does_not_need_a_configured_shell() {
    // e.g. a CI step: no `icicle env`, no ICICLE_SHELL_PATH
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");

    let out = sb.raw("icicle exec --using 2024-01-01 yosys").success();
    assert_eq!(out, "yosys-2024-01-01");
}

#[test]
fn exec_uses_the_version_file_then_the_default() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");
    sb.add_toolchain("2024-02-02");
    sb.sh("icicle default 2024-02-02").success();

    // no file: the default
    assert_eq!(sb.raw("icicle exec yosys").success(), "yosys-2024-02-02");

    // a file in a parent directory wins over the default
    fs::write(sb.work.join(".icicle-toolchain"), "2024-01-01").unwrap();
    fs::create_dir_all(sb.work.join("sub")).unwrap();
    assert_eq!(
        sb.raw("cd sub && icicle exec yosys").success(),
        "yosys-2024-01-01"
    );

    // an explicit version wins over the file
    assert_eq!(
        sb.raw("icicle exec --using 2024-02-02 yosys").success(),
        "yosys-2024-02-02"
    );
}

#[test]
fn exec_passes_arguments_and_exit_codes_through() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");

    let out =
        sb.raw("icicle exec --using 2024-01-01 sh -c 'echo \"$1 $2\"; exit 7' sh --flag 'a b'");
    assert_eq!(out.stdout, "--flag a b");
    assert!(!out.ok);

    let code = sb
        .raw("icicle exec --using 2024-01-01 sh -c 'exit 7'; echo $?")
        .success();
    assert_eq!(code, "7");
}

#[test]
fn exec_reports_problems_clearly() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");

    let out = sb.raw("icicle exec yosys");
    assert!(!out.ok);
    assert!(
        out.stderr.contains("No version specified"),
        "{}",
        out.stderr
    );

    let out = sb.raw("icicle exec --using 1999-01-01 yosys");
    assert!(!out.ok);
    assert!(
        out.stderr.contains("Unable to find toolchain"),
        "{}",
        out.stderr
    );

    let out = sb.raw("icicle exec --using 2024-01-01 no-such-tool");
    assert!(!out.ok);
    assert!(
        out.stderr.contains("Failed to run 'no-such-tool'"),
        "{}",
        out.stderr
    );

    let out = sb.raw("icicle exec --using ../../etc yosys");
    assert!(!out.ok);
    assert!(
        out.stderr.contains("not a valid toolchain version"),
        "{}",
        out.stderr
    );
}

// ---- session lifetime ----------------------------------------------------------------------

fn backdate(link: &Path, timestamp: &str) {
    assert!(Command::new("touch")
        .args(["-h", "-t", timestamp])
        .arg(link)
        .status()
        .unwrap()
        .success());
}

#[test]
fn session_links_record_the_owning_shell() {
    let sb = Sandbox::new();
    let out = sb.sh(r#"basename "$ICICLE_SHELL_PATH""#).success();
    let mut parts = out.splitn(3, '_');
    assert_eq!(parts.next(), Some("icicle"));
    assert!(parts.next().unwrap().parse::<u32>().is_ok(), "{out}");
    assert!(parts.next().is_some(), "{out}");
}

#[test]
fn old_session_links_survive_while_their_shell_is_alive() {
    let sb = Sandbox::new();
    let caches = sb.home.join("caches");
    fs::create_dir_all(&caches).unwrap();

    let alive = caches.join(format!("icicle_{}_alive", std::process::id()));
    let dead = caches.join("icicle_4194305_dead");
    let legacy = caches.join("icicle_d3b07384-d9a0-4c8e-8f3a-1b2c3d4e5f60");
    for link in [&alive, &dead, &legacy] {
        unix_fs::symlink("/nonexistent", link).unwrap();
        backdate(link, "202001010000");
    }

    sb.sh("true").success();
    assert!(
        alive.symlink_metadata().is_ok(),
        "a live shell's link must be kept"
    );
    assert!(
        dead.symlink_metadata().is_err(),
        "a dead shell's old link should go"
    );
    assert!(
        legacy.symlink_metadata().is_err(),
        "an old link with no pid should go"
    );
}

#[test]
fn recent_session_links_are_kept_even_when_their_shell_is_gone() {
    let sb = Sandbox::new();
    let caches = sb.home.join("caches");
    fs::create_dir_all(&caches).unwrap();
    let recent = caches.join("icicle_4194305_recent");
    unix_fs::symlink("/nonexistent", &recent).unwrap();

    sb.sh("true").success();
    assert!(recent.symlink_metadata().is_ok());
}

#[test]
fn running_icicle_refreshes_its_own_session_link() {
    let sb = Sandbox::new();
    sb.add_toolchain("2024-01-01");

    let out = sb
        .sh(r#"
            touch -h -t 202001010000 "$ICICLE_SHELL_PATH"
            icicle use 2024-01-01 >/dev/null
            touch -h -t 202001010000 "$ICICLE_SHELL_PATH"
            icicle list >/dev/null
            echo "$ICICLE_SHELL_PATH"
        "#)
        .success();
    let link = PathBuf::from(out);
    let age = std::time::SystemTime::now()
        .duration_since(link.symlink_metadata().unwrap().modified().unwrap())
        .unwrap();
    assert!(age.as_secs() < 3600, "link was not refreshed: {age:?}");
    // and it still points at the same toolchain
    assert!(fs::read_link(&link)
        .unwrap()
        .ends_with("toolchains/2024-01-01"));
}

// ---- install -------------------------------------------------------------------------------

/// Serve `routes` (path -> body) on localhost and return the base url
fn serve(routes: Vec<(String, Vec<u8>)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);
            let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
            let body = routes
                .iter()
                .find(|(p, _)| *p == path)
                .map(|(_, b)| b.clone());
            let _ = match body {
                Some(body) => {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    stream.write_all(&body)
                }
                None => write!(
                    stream,
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                ),
            };
        }
    });
    url
}

fn platform() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    (os, arch)
}

fn route(version: &str) -> String {
    let (os, arch) = platform();
    format!(
        "/{}/oss-cad-suite-{}-{}-{}.tgz",
        version,
        os,
        arch,
        version.replace('-', "")
    )
}

fn fake_tarball(version: &str) -> Vec<u8> {
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        let script = format!("#!/bin/sh\necho yosys-{}\n", version);
        let mut header = tar::Header::new_gnu();
        header.set_size(script.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, "oss-cad-suite/bin/yosys", script.as_bytes())
            .unwrap();
        builder.finish().unwrap();
    }
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    gz.write_all(&tar_bytes).unwrap();
    gz.finish().unwrap()
}

fn with_mirror(sb: &Sandbox, mirror: &str, script: &str) -> Output {
    sb.sh(&format!(
        "export ICICLE_DIST_MIRROR='{}'\n{}",
        mirror, script
    ))
}

#[test]
fn install_then_use() {
    let sb = Sandbox::new();
    let mirror = serve(vec![(route("2024-01-01"), fake_tarball("2024-01-01"))]);

    let out = with_mirror(
        &sb,
        &mirror,
        "icicle install 2024-01-01 >/dev/null && icicle use 2024-01-01 >/dev/null && yosys",
    )
    .success();
    assert_eq!(out, "yosys-2024-01-01");
    assert_eq!(sb.sh("icicle list").success(), "* 2024-01-01");

    // installing again is a no-op
    let again = with_mirror(&sb, &mirror, "icicle install 2024-01-01").success();
    assert!(again.contains("already installed"), "{again}");
}

#[test]
fn install_announces_the_size_without_control_characters_when_not_a_terminal() {
    let sb = Sandbox::new();
    let mirror = serve(vec![(route("2024-01-01"), fake_tarball("2024-01-01"))]);

    let out = with_mirror(&sb, &mirror, "icicle install 2024-01-01");
    assert!(out.ok, "{}", out.stderr);
    assert!(out.stderr.contains("Downloading "), "{}", out.stderr);
    assert!(
        !out.stderr.contains('\r') && !out.stderr.contains('\x1b'),
        "progress must not draw when stderr is not a terminal: {:?}",
        out.stderr
    );
}

#[test]
fn install_reads_the_toolchain_file() {
    let sb = Sandbox::new();
    let mirror = serve(vec![(route("2024-01-01"), fake_tarball("2024-01-01"))]);
    fs::write(sb.work.join(".icicle-toolchain"), "2024-01-01\n").unwrap();

    with_mirror(&sb, &mirror, "icicle install").success();
    assert!(sb
        .home
        .join("toolchains/2024-01-01/oss-cad-suite/bin/yosys")
        .exists());
}

#[test]
fn install_of_a_missing_version_reports_it_and_leaves_nothing_behind() {
    let sb = Sandbox::new();
    let mirror = serve(vec![]);

    let out = with_mirror(&sb, &mirror, "icicle install 2024-01-01");
    assert!(!out.ok);
    assert!(out.stderr.contains("not found"), "{}", out.stderr);
    assert_eq!(sb.sh("icicle list").success(), "");
}

#[test]
fn a_corrupt_download_is_not_mistaken_for_an_installed_toolchain() {
    let sb = Sandbox::new();
    let mut tarball = fake_tarball("2024-01-01");
    tarball.truncate(tarball.len() / 2);
    let mirror = serve(vec![(route("2024-01-01"), tarball)]);

    assert!(!with_mirror(&sb, &mirror, "icicle install 2024-01-01").ok);
    assert_eq!(sb.sh("icicle list").success(), "");
    let leftovers: Vec<_> = fs::read_dir(sb.home.join("toolchains")).unwrap().collect();
    assert!(
        leftovers.is_empty(),
        "partial install left behind: {leftovers:?}"
    );

    // a retry against a healthy mirror works
    let mirror = serve(vec![(route("2024-01-01"), fake_tarball("2024-01-01"))]);
    with_mirror(&sb, &mirror, "icicle install 2024-01-01").success();
    assert_eq!(sb.sh("icicle list").success(), "* 2024-01-01");
}
