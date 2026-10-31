//! Tests for `.ci/install.sh`, run against a fake `$HOME` with the binary "downloaded" from a
//! local file.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct Machine {
    root: PathBuf,
    home: PathBuf,
}

impl Machine {
    fn new() -> Machine {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root =
            std::env::temp_dir().join(format!("icicle-install-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&root);
        let home = root.join("home");
        fs::create_dir_all(&home).unwrap();
        fs::write(root.join("icicle-bin"), "#!/bin/sh\n").unwrap();
        Machine { root, home }
    }

    fn install(&self, shell: &str, extra: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/.ci/install.sh");
        let mut cmd = Command::new("bash");
        cmd.arg(script)
            .args(extra)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", &self.home)
            .env("SHELL", shell)
            .env(
                "ICICLE_DOWNLOAD_URL",
                format!("file://{}", self.root.join("icicle-bin").display()),
            );
        for (k, v) in envs {
            cmd.env(k, v);
        }
        cmd.output().unwrap()
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.home.join(relative))
            .unwrap_or_else(|_| panic!("{relative} was not created"))
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn assert_ok(out: &std::process::Output) {
    assert!(
        out.status.success(),
        "install failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn blocks(content: &str) -> usize {
    content
        .matches("# Icicle https://github.com/nishtahir/icicle/")
        .count()
}

#[test]
fn installs_the_binary() {
    let m = Machine::new();
    assert_ok(&m.install("/bin/zsh", &[], &[]));
    let bin = m.home.join(".icicle/icicle");
    assert!(bin.exists());
    assert_ne!(fs::metadata(bin).unwrap().permissions().mode() & 0o100, 0);
}

#[test]
fn zsh_is_configured_once_even_if_installed_twice() {
    let m = Machine::new();
    assert_ok(&m.install("/bin/zsh", &[], &[]));
    assert_ok(&m.install("/bin/zsh", &[], &[]));

    let zshrc = m.read(".zshrc");
    assert_eq!(blocks(&zshrc), 1, "{zshrc}");
    assert!(zshrc.contains(&format!("export PATH={}/.icicle:$PATH", m.home.display())));
    assert!(zshrc.contains(r#"eval "$(icicle env)""#));
}

#[test]
fn existing_rc_content_is_preserved() {
    let m = Machine::new();
    fs::write(m.home.join(".zshrc"), "export FOO=bar\n").unwrap();
    assert_ok(&m.install("/bin/zsh", &[], &[]));
    assert!(m.read(".zshrc").starts_with("export FOO=bar\n"));
}

#[test]
fn zsh_honours_zdotdir() {
    let m = Machine::new();
    let zdotdir = m.root.join("zdot");
    assert_ok(&m.install("/bin/zsh", &[], &[("ZDOTDIR", zdotdir.to_str().unwrap())]));
    assert_eq!(
        blocks(&fs::read_to_string(zdotdir.join(".zshrc")).unwrap()),
        1
    );
    assert!(!m.home.join(".zshrc").exists());
}

#[test]
fn fish_uses_the_directory_fish_reads() {
    let m = Machine::new();
    assert_ok(&m.install("/usr/bin/fish", &[], &[]));
    let conf = m.read(".config/fish/conf.d/icicle.fish");
    assert!(conf.contains("icicle env | source"));
    assert!(conf.contains(&format!("set PATH {}/.icicle $PATH", m.home.display())));
}

#[cfg(target_os = "macos")]
mod macos_bash {
    use super::*;

    // macOS terminals start login shells, which read the first of these that exists

    #[test]
    fn creates_bash_profile_when_nothing_exists() {
        let m = Machine::new();
        assert_ok(&m.install("/bin/bash", &[], &[]));
        assert_eq!(blocks(&m.read(".bash_profile")), 1);
        assert!(!m.home.join(".bashrc").exists());
    }

    #[test]
    fn prefers_an_existing_bash_profile_over_profile() {
        let m = Machine::new();
        fs::write(m.home.join(".bash_profile"), "").unwrap();
        fs::write(m.home.join(".profile"), "").unwrap();
        assert_ok(&m.install("/bin/bash", &[], &[]));
        assert_eq!(blocks(&m.read(".bash_profile")), 1);
        assert_eq!(blocks(&m.read(".profile")), 0);
    }

    #[test]
    fn uses_profile_when_it_is_the_only_one() {
        let m = Machine::new();
        fs::write(m.home.join(".profile"), "").unwrap();
        assert_ok(&m.install("/bin/bash", &[], &[]));
        assert_eq!(blocks(&m.read(".profile")), 1);
        assert!(!m.home.join(".bash_profile").exists());
    }
}

#[cfg(target_os = "linux")]
#[test]
fn bash_uses_bashrc_on_linux() {
    let m = Machine::new();
    assert_ok(&m.install("/bin/bash", &[], &[]));
    assert_eq!(blocks(&m.read(".bashrc")), 1);
}

#[test]
fn skip_shell_leaves_rc_files_alone() {
    let m = Machine::new();
    assert_ok(&m.install("/bin/zsh", &["--skip-shell"], &[]));
    assert!(m.home.join(".icicle/icicle").exists());
    assert!(!m.home.join(".zshrc").exists());
}

#[test]
fn custom_install_dir_is_used_for_the_binary_and_path() {
    let m = Machine::new();
    let dir = m.root.join("custom dir");
    assert_ok(&m.install("/bin/zsh", &["--install-dir", dir.to_str().unwrap()], &[]));
    assert!(dir.join("icicle").exists());
    assert!(m
        .read(".zshrc")
        .contains(&format!("export PATH={}:$PATH", dir.display())));
}

#[test]
fn unsupported_shell_prints_instructions_instead_of_failing() {
    let m = Machine::new();
    let out = m.install("/bin/tcsh", &[], &[]);
    assert_ok(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("Unsupported shell 'tcsh'"));
}

#[test]
fn failed_download_fails_the_install() {
    let m = Machine::new();
    let out = m.install(
        "/bin/zsh",
        &[],
        &[("ICICLE_DOWNLOAD_URL", "file:///nonexistent")],
    );
    assert!(!out.status.success());
    assert!(!m.home.join(".zshrc").exists());
}
