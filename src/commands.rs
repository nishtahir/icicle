use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::process::{parent_id, CommandExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use uuid::Uuid;

use crate::environment::{read_version_file, resolve_version, validate_version, Environment, Home};
use crate::fs_util::replace_symlink;
use crate::install;
use crate::progress::format_bytes;

/// Session links in caches that haven't been touched for this long are removed. This was an
/// arbitrary decision but don't want to fill up the user's disk with old symlinks.
const CACHE_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 7);

pub fn install(version: Option<&str>) -> Result<()> {
    let env = Environment::create()?;
    let version = resolve_version(&env::current_dir()?, version)?;
    install::install(&env, &version)
}

pub fn latest() -> Result<()> {
    let env = Environment::create()?;
    let version = install::resolve_latest_version()?;
    println!("Latest version is {}", version);
    install::install(&env, &version)
}

pub fn uninstall(version: &str) -> Result<()> {
    let env = Environment::create()?;
    validate_version(version)?;

    let toolchain_path = env.home.toolchains.join(version);
    if !toolchain_path.is_dir() {
        bail!("'{}' is not a valid toolchain directory.", version);
    }

    fs::remove_dir_all(&toolchain_path).with_context(|| {
        format!(
            "Failed to remove toolchain directory '{}'",
            toolchain_path.display()
        )
    })?;

    // don't leave the default pointing at something that no longer exists
    let default_alias = env.home.default_alias();
    if default_alias.symlink_metadata().is_ok() && !default_alias.exists() {
        fs::remove_file(&default_alias).with_context(|| "Failed to remove dangling default")?;
        println!("Removed default because '{}' was uninstalled.", version);
    }

    Ok(())
}

pub fn set_default(version: &str) -> Result<()> {
    let env = Environment::create()?;
    validate_version(version)?;

    let toolchain_path = env.home.toolchains.join(version);
    if !toolchain_path.is_dir() {
        bail!("'{}' is not a valid toolchain directory.", version);
    }

    println!("Setting {} as default.", version);
    replace_symlink(&toolchain_path, &env.home.default_alias())
}

pub fn use_toolchain(version: Option<&str>) -> Result<()> {
    let env = Environment::create()?;
    let version = resolve_version(&env::current_dir()?, version)?;

    let toolchain = env.home.find_toolchain(&version)?;

    // TODO - handle windows
    replace_symlink(&toolchain, &env.shell_path)?;
    println!("Using toolchain {}", version);
    Ok(())
}

/// The toolchain to run something with: an explicit version, then the nearest
/// `.icicle-toolchain`, then the default
fn select_toolchain(home: &Home, using: Option<&str>) -> Result<PathBuf> {
    if let Some(version) = using {
        return home.find_toolchain(version);
    }
    if let Some(version) = read_version_file(&env::current_dir()?)? {
        return home.find_toolchain(&version);
    }
    home.find_toolchain("default").context(
        "No version specified. Use '--using <version>', create a '.icicle-toolchain' file or set a default with 'icicle default <version>'.",
    )
}

/// Run a command with a toolchain's tools on the `PATH`, without touching the shell session.
/// Works outside of an `icicle env` shell, so it suits scripts and CI.
pub fn exec(using: Option<&str>, command: &[String]) -> Result<()> {
    let home = Home::locate()?;
    let (program, args) = command.split_first().context("No command specified.")?;

    let toolchain = select_toolchain(&home, using)?;

    let suite = toolchain.join("oss-cad-suite");
    let mut entries = vec![suite.join("bin"), suite.join("libexec")];
    entries.extend(env::split_paths(&env::var_os("PATH").unwrap_or_default()));
    let path = env::join_paths(entries).context("Failed to build PATH")?;

    // exec replaces this process, so the command's exit code and signals pass straight through.
    // std resolves `program` against the PATH set here.
    let error = std::process::Command::new(program)
        .args(args)
        .env("PATH", path)
        .exec();
    Err(error).with_context(|| format!("Failed to run '{}'", program))
}

pub fn current() -> Result<()> {
    let env = Environment::create()?;
    println!("{}", current_version(&env.home, &env.shell_path)?);
    Ok(())
}

/// Name of the toolchain the session link at `shell_path` resolves to
pub fn current_version(home: &Home, shell_path: &Path) -> Result<String> {
    let resolved = fs::canonicalize(shell_path).with_context(|| {
        "Failed to resolve toolchain. You may need to set a toolchain with 'icicle use <toolchain>'"
    })?;

    toolchain_name(home, &resolved)
        .with_context(|| format!("Failed to determine toolchain for '{}'", resolved.display()))
}

/// The toolchain a fully resolved path is in. The home itself may sit behind a symlink (e.g.
/// /tmp on macOS), so callers must pass a canonical path, which is compared with the
/// canonical toolchains directory.
fn toolchain_name(home: &Home, resolved: &Path) -> Option<String> {
    let toolchains = fs::canonicalize(&home.toolchains).ok()?;
    let name = match resolved.strip_prefix(&toolchains) {
        Ok(relative) => relative.components().next().map(|c| c.as_os_str()),
        Err(_) => resolved.file_name(),
    };
    name.map(|name| name.to_string_lossy().to_string())
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Listed {
    pub name: String,
    pub is_default: bool,
    pub is_current: bool,
}

pub fn list() -> Result<()> {
    let env = Environment::create()?;
    for toolchain in list_toolchains(&env.home, Some(&env.shell_path))? {
        let tags: Vec<&str> = [
            toolchain.is_default.then_some("default"),
            toolchain.is_current.then_some("current"),
        ]
        .into_iter()
        .flatten()
        .collect();

        if tags.is_empty() {
            println!("* {}", toolchain.name);
        } else {
            println!("* {} ({})", toolchain.name, tags.join(", "));
        }
    }
    Ok(())
}

/// Installed toolchains, oldest first. `shell_path` is this shell's session link, if any.
pub fn list_toolchains(home: &Home, shell_path: Option<&Path>) -> Result<Vec<Listed>> {
    // The user might not have a default, or a session, set
    let default = fs::canonicalize(home.default_alias()).ok();
    let current = shell_path.and_then(|p| fs::canonicalize(p).ok());

    let entries = fs::read_dir(&home.toolchains).with_context(|| {
        format!(
            "Failed to read toolchain directory '{}'",
            home.toolchains.display()
        )
    })?;

    let mut toolchains = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }
        let resolved = fs::canonicalize(&path).ok();
        toolchains.push(Listed {
            name,
            is_default: resolved.is_some() && resolved == default,
            is_current: resolved.is_some() && resolved == current,
        });
    }
    toolchains.sort();
    Ok(toolchains)
}

/// Print where a tool comes from: `--using`, else what this shell would run, else what
/// `exec` would run.
pub fn which(tool: &str, using: Option<&str>) -> Result<()> {
    let home = Home::locate()?;
    if tool.is_empty() || tool.contains('/') {
        bail!("'{}' is not a valid tool name.", tool);
    }

    let session = match using {
        Some(_) => None,
        None => env::var_os("ICICLE_SHELL_PATH").and_then(|p| fs::canonicalize(p).ok()),
    };
    let toolchain = match session {
        Some(toolchain) => toolchain,
        // resolve aliases such as `default` so the path stays valid if they are repointed
        None => fs::canonicalize(select_toolchain(&home, using)?)
            .context("Failed to resolve toolchain.")?,
    };

    let suite = toolchain.join("oss-cad-suite");
    let found = ["bin", "libexec"]
        .iter()
        .map(|dir| suite.join(dir).join(tool))
        .find(|candidate| is_executable(candidate));

    match found {
        Some(path) => {
            println!("{}", path.display());
            Ok(())
        }
        None => bail!(
            "'{}' was not found in toolchain '{}'.",
            tool,
            toolchain_name(&home, &toolchain).unwrap_or_else(|| toolchain.display().to_string())
        ),
    }
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

pub struct PrunePlan {
    /// Toolchains that stay, and why
    pub keep: Vec<(String, Vec<&'static str>)>,
    /// Toolchains to delete, with the space they take
    pub remove: Vec<(String, u64)>,
    /// Leftovers of interrupted installs
    pub partial: Vec<(PathBuf, u64)>,
}

/// Work out what `prune` would delete. Everything goes except the default, the toolchain this
/// shell is using, and any toolchain another still-running shell is using.
pub fn prune_plan(home: &Home, shell_path: Option<&Path>) -> Result<PrunePlan> {
    let mut reasons: std::collections::BTreeMap<String, Vec<&'static str>> = Default::default();
    let mut keep_for = |resolved: Option<PathBuf>, reason: &'static str| {
        if let Some(name) = resolved.and_then(|p| toolchain_name(home, &p)) {
            reasons.entry(name).or_default().push(reason);
        }
    };

    keep_for(fs::canonicalize(home.default_alias()).ok(), "default");
    keep_for(
        shell_path.and_then(|p| fs::canonicalize(p).ok()),
        "in use by this shell",
    );

    let own_session = shell_path.and_then(|p| fs::canonicalize(p).ok());
    if let Ok(entries) = fs::read_dir(&home.caches) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let target = fs::canonicalize(entry.path()).ok();
            // this shell's own link is already counted
            if target.is_some() && target == own_session {
                continue;
            }
            if session_is_alive(&name) {
                keep_for(target, "in use by another shell");
            }
        }
    }

    let mut keep = Vec::new();
    let mut remove = Vec::new();
    let mut partial = Vec::new();
    for entry in fs::read_dir(&home.toolchains)
        .with_context(|| format!("Failed to read '{}'", home.toolchains.display()))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if name.starts_with('.') {
            if name.ends_with(".partial") {
                partial.push((path.clone(), dir_size(&path)));
            }
            continue;
        }
        match reasons.remove(&name) {
            Some(why) => keep.push((name, why)),
            None => remove.push((name, dir_size(&path))),
        }
    }
    keep.sort();
    remove.sort();
    partial.sort();
    Ok(PrunePlan {
        keep,
        remove,
        partial,
    })
}

/// Total size of the files below `path`, not following symlinks
fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(m) if m.is_dir() => dir_size(&entry.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

pub fn prune(dry_run: bool, yes: bool) -> Result<()> {
    let home = Home::locate()?;
    if !home.toolchains.is_dir() {
        println!("Nothing to prune.");
        return Ok(());
    }
    let shell_path = env::var_os("ICICLE_SHELL_PATH").map(PathBuf::from);
    let plan = prune_plan(&home, shell_path.as_deref())?;

    for (name, why) in &plan.keep {
        println!("Keeping  {} ({})", name, why.join(", "));
    }
    for (name, size) in &plan.remove {
        println!("Removing {} ({})", name, format_bytes(*size));
    }
    for (path, size) in &plan.partial {
        println!(
            "Removing {} (unfinished install, {})",
            path.file_name().unwrap_or_default().to_string_lossy(),
            format_bytes(*size)
        );
    }

    let total: u64 = plan
        .remove
        .iter()
        .map(|(_, size)| size)
        .chain(plan.partial.iter().map(|(_, size)| size))
        .sum();
    if plan.remove.is_empty() && plan.partial.is_empty() {
        println!("Nothing to prune.");
        return Ok(());
    }
    if dry_run {
        println!(
            "Dry run: would free {}. Nothing was removed.",
            format_bytes(total)
        );
        return Ok(());
    }
    if !yes && !confirm(&format!("Free {}?", format_bytes(total)))? {
        println!("Aborted.");
        return Ok(());
    }

    let paths = plan
        .remove
        .iter()
        .map(|(name, _)| home.toolchains.join(name))
        .chain(plan.partial.iter().map(|(path, _)| path.clone()));
    for path in paths {
        fs::remove_dir_all(&path)
            .with_context(|| format!("Failed to remove '{}'", path.display()))?;
    }
    println!("Freed {}.", format_bytes(total));
    Ok(())
}

/// Ask on the terminal. Refuses to guess when there is nobody to ask.
fn confirm(question: &str) -> Result<bool> {
    use std::io::{BufRead, IsTerminal, Write};

    if !std::io::stdin().is_terminal() {
        bail!("Refusing to remove toolchains without confirmation. Pass '--yes' to proceed.");
    }
    print!("{} [y/N] ", question);
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "y" | "yes"))
}

pub fn env_command() -> Result<()> {
    // The user is trying to configure their shell to use icicle
    // We don't expect them to have ICICLE_HOME set yet
    let home = Home::locate()?;
    home.create_dirs()?;

    let previous = env::var_os("ICICLE_SHELL_PATH").map(PathBuf::from);
    let path = env::var_os("PATH").unwrap_or_default();
    print!("{}", render_env(&home, previous.as_deref(), &path)?);
    Ok(())
}

/// Create a session link and return the shell script that activates it.
///
/// Each shell gets its own link so `icicle use` only affects that shell. When this runs in a
/// shell that already has a session (nested shells, a re-sourced profile), the new link starts
/// out on the same toolchain and the old session's `PATH` entries are replaced rather than
/// piled up - otherwise the first `PATH` entry would be a link that `ICICLE_SHELL_PATH` and
/// therefore `icicle use` no longer controls.
pub fn render_env(home: &Home, previous: Option<&Path>, path: &OsString) -> Result<String> {
    cleanup_old_caches(&home.caches);

    let inherited = previous
        .filter(|p| p.parent() == Some(home.caches.as_path()))
        .and_then(|p| fs::read_link(p).ok());
    let target = inherited.unwrap_or_else(|| home.default_alias());

    // `icicle env` runs as `eval "$(icicle env)"`, so our parent is normally the shell. Recording
    // it lets cleanup tell a shell that is still open from one that is long gone.
    let session = home
        .caches
        .join(format!("icicle_{}_{}", parent_id(), Uuid::new_v4()));
    replace_symlink(&target, &session)?;

    let mut entries = vec![
        session.join("oss-cad-suite/bin"),
        session.join("oss-cad-suite/libexec"),
    ];
    entries.extend(env::split_paths(path).filter(|p| !p.starts_with(&home.caches)));
    let path = env::join_paths(entries).context("Failed to build PATH")?;

    Ok(format!(
        "export ICICLE_SHELL_PATH={}\nexport ICICLE_HOME={}\nexport PATH={}\n",
        shell_quote(&session.to_string_lossy()),
        shell_quote(&home.root.to_string_lossy()),
        shell_quote(&path.to_string_lossy()),
    ))
}

/// Single quote a value so it survives `eval` with spaces or other special characters in it
fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Best effort. Failing to tidy up must never stop a shell from starting.
///
/// A link is removed when it hasn't been used for a week and its shell has gone. Timestamps
/// alone can't be trusted: using a tool through a symlink does not update the link's times on
/// every filesystem (macOS for one), so a shell that has been open for weeks looks idle.
fn cleanup_old_caches(caches: &Path) {
    let Ok(entries) = fs::read_dir(caches) else {
        return;
    };

    let now = SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("icicle_") {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let last_used = [metadata.accessed().ok(), metadata.modified().ok()]
            .into_iter()
            .flatten()
            .max();
        let idle = last_used.and_then(|t| now.duration_since(t).ok());

        let is_stale = idle.is_some_and(|idle| idle > CACHE_MAX_AGE);
        if is_stale && !session_is_alive(&name) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// The pid recorded in a session link name (`icicle_<pid>_<uuid>`), if it is still running.
/// Links without one, from older versions, are never considered alive.
fn session_is_alive(name: &str) -> bool {
    let pid = name
        .strip_prefix("icicle_")
        .and_then(|rest| rest.split_once('_'))
        .and_then(|(pid, _)| pid.parse::<i32>().ok());

    let Some(pid) = pid.filter(|pid| *pid > 0) else {
        return false;
    };

    // signal 0 only checks that the process exists; EPERM means it exists but isn't ours
    // SAFETY: kill with signal 0 sends nothing
    let exists = unsafe { libc::kill(pid, 0) } == 0;
    exists || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Keep the timestamps on this shell's own link fresh whenever icicle runs from it
pub fn refresh_session(home: &Home, shell_path: &Path) {
    const REFRESH_AFTER: Duration = Duration::from_secs(60 * 60 * 24);

    if shell_path.parent() != Some(home.caches.as_path()) {
        return;
    }
    let Ok(metadata) = shell_path.symlink_metadata() else {
        return;
    };
    let age = metadata
        .modified()
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok());

    if age.is_some_and(|age| age > REFRESH_AFTER) {
        if let Ok(target) = fs::read_link(shell_path) {
            let _ = replace_symlink(&target, shell_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_liveness_follows_the_recorded_pid() {
        let me = std::process::id();
        assert!(session_is_alive(&format!("icicle_{me}_abc")));
        // pid 2^22 + 1 is above the default pid limit on Linux and macOS
        assert!(!session_is_alive("icicle_4194305_abc"));
        // old style names have no pid
        assert!(!session_is_alive(
            "icicle_d3b07384-d9a0-4c8e-8f3a-1b2c3d4e5f60"
        ));
        assert!(!session_is_alive("icicle_0_abc"));
        assert!(!session_is_alive("icicle_-1_abc"));
        assert!(!session_is_alive("garbage"));
    }

    #[test]
    fn quotes_awkward_values() {
        assert_eq!(shell_quote("/a b/c"), "'/a b/c'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }
}
