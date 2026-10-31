use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use uuid::Uuid;

use crate::environment::{resolve_version, validate_version, Environment, Home};
use crate::fs_util::replace_symlink;
use crate::install;

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

    let toolchain = [
        env.home.toolchains.join(&version),
        env.home.aliases.join(&version),
    ]
    .into_iter()
    .find(|path| path.exists())
    .context("Unable to find toolchain. Did you install it with 'icicle install'?")?;

    if !toolchain.is_dir() {
        bail!("'{}' is not a valid toolchain directory.", version);
    }

    // TODO - handle windows
    replace_symlink(&toolchain, &env.shell_path)?;
    println!("Using toolchain {}", version);
    Ok(())
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

    // the home itself may sit behind a symlink (e.g. /tmp on macOS), so compare resolved paths
    let toolchains = fs::canonicalize(&home.toolchains)
        .with_context(|| format!("Failed to resolve '{}'", home.toolchains.display()))?;

    let name = match resolved.strip_prefix(&toolchains) {
        Ok(relative) => relative.components().next().map(|c| c.as_os_str()),
        Err(_) => resolved.file_name(),
    };

    name.map(|name| name.to_string_lossy().to_string())
        .with_context(|| format!("Failed to determine toolchain for '{}'", resolved.display()))
}

pub fn list() -> Result<()> {
    let env = Environment::create()?;
    for (name, is_default) in list_toolchains(&env.home)? {
        let suffix = if is_default { " (default)" } else { "" };
        println!("* {}{}", name, suffix);
    }
    Ok(())
}

pub fn list_toolchains(home: &Home) -> Result<Vec<(String, bool)>> {
    // The user might not have a default set
    let default = fs::canonicalize(home.default_alias()).ok();

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
        let is_default = default.is_some() && fs::canonicalize(&path).ok() == default;
        toolchains.push((name, is_default));
    }
    toolchains.sort();
    Ok(toolchains)
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

    let session = home.caches.join(format!("icicle_{}", Uuid::new_v4()));
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
fn cleanup_old_caches(caches: &Path) {
    let Ok(entries) = fs::read_dir(caches) else {
        return;
    };

    let now = SystemTime::now();
    for entry in entries.flatten() {
        let is_session = entry.file_name().to_string_lossy().starts_with("icicle_");
        let age = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok());

        if let (true, Some(age)) = (is_session, age) {
            if age > CACHE_MAX_AGE {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_awkward_values() {
        assert_eq!(shell_quote("/a b/c"), "'/a b/c'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }
}
