use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

pub const TOOLCHAIN_FILE: &str = ".icicle-toolchain";
pub const DEFAULT_DIST_MIRROR: &str =
    "https://github.com/YosysHQ/oss-cad-suite-build/releases/download";

/// Directory layout of an icicle home
#[derive(Debug, Clone)]
pub struct Home {
    pub root: PathBuf,
    pub aliases: PathBuf,
    pub toolchains: PathBuf,
    pub caches: PathBuf,
}

impl Home {
    pub fn new(root: impl Into<PathBuf>) -> Home {
        let root = root.into();
        Home {
            aliases: root.join("aliases"),
            toolchains: root.join("toolchains"),
            caches: root.join("caches"),
            root,
        }
    }

    /// `ICICLE_HOME` if set, otherwise `~/.icicle`
    pub fn locate() -> Result<Home> {
        let root = match env::var_os("ICICLE_HOME") {
            Some(home) => PathBuf::from(home),
            None => PathBuf::from(env::var_os("HOME").context("Failed to get home directory")?)
                .join(".icicle"),
        };
        Ok(Home::new(root))
    }

    pub fn default_alias(&self) -> PathBuf {
        self.aliases.join("default")
    }

    pub fn create_dirs(&self) -> Result<()> {
        for dir in [&self.root, &self.aliases, &self.toolchains, &self.caches] {
            fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory '{}'", dir.display()))?;
        }
        Ok(())
    }
}

/// Everything a command needs, resolved from the process environment
pub struct Environment {
    pub home: Home,
    pub shell_path: PathBuf,
    pub os: &'static str,
    pub arch: &'static str,
    pub dist_mirror: String,
}

impl Environment {
    pub fn create() -> Result<Environment> {
        let home = env::var_os("ICICLE_HOME").map(Home::new).context(
            "'ICICLE_HOME' environment variable not set. Did you setup your shell to 'eval $(icicle env)'?",
        )?;

        let shell_path = env::var_os("ICICLE_SHELL_PATH").map(PathBuf::from).context(
            "'ICICLE_SHELL_PATH' environment variable not set. Did you setup your shell to 'eval $(icicle env)'?",
        )?;

        let (os, arch) = platform()?;
        home.create_dirs()?;

        Ok(Environment {
            home,
            shell_path,
            os,
            arch,
            dist_mirror: env::var("ICICLE_DIST_MIRROR")
                .map(|m| m.trim_end_matches('/').to_string())
                .unwrap_or_else(|_| DEFAULT_DIST_MIRROR.to_string()),
        })
    }
}

fn platform() -> Result<(&'static str, &'static str)> {
    let os = match env::consts::OS {
        "linux" => "linux",
        "macos" => "darwin",
        other => bail!("Unsupported OS '{}'", other),
    };

    let arch = match env::consts::ARCH {
        "x86_64" | "amd64" => "x64",
        "aarch64" => "arm64",
        other => bail!("Unsupported architecture '{}'", other),
    };

    Ok((os, arch))
}

/// Toolchain names end up in filesystem paths, so they must not be able to escape the
/// toolchains directory (e.g. `icicle uninstall ../..`).
pub fn validate_version(version: &str) -> Result<()> {
    if version.is_empty() || version.starts_with('.') || version.contains(['/', '\\', '\0']) {
        bail!("'{}' is not a valid toolchain version.", version);
    }
    Ok(())
}

/// The explicit version if given, otherwise the contents of `.icicle-toolchain` in `dir`
pub fn resolve_version(dir: &Path, version: Option<&str>) -> Result<String> {
    if let Some(version) = version {
        validate_version(version)?;
        return Ok(version.to_string());
    }

    let file = dir.join(TOOLCHAIN_FILE);
    let content = match fs::read_to_string(&file) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!(
            "No version specified. Please specify a version or create a \
            '{}' file with the version you want to use.",
            TOOLCHAIN_FILE
        ),
        Err(e) => {
            return Err(e).with_context(|| format!("Failed to read file '{}'", file.display()))
        }
    };

    let version = content.trim();
    if version.is_empty() {
        bail!("'{}' is empty.", file.display());
    }
    validate_version(version)?;
    Ok(version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal() {
        for bad in ["", ".", "..", "../..", "a/b", "/etc", ".hidden", "a\\b"] {
            assert!(validate_version(bad).is_err(), "{bad:?} should be rejected");
        }
        assert!(validate_version("2024-01-01").is_ok());
    }

    #[test]
    fn explicit_version_wins_over_file() {
        let dir = std::env::temp_dir().join(format!("icicle-ut-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(TOOLCHAIN_FILE), "2020-01-01\n").unwrap();
        assert_eq!(
            resolve_version(&dir, Some("2024-01-01")).unwrap(),
            "2024-01-01"
        );
        assert_eq!(resolve_version(&dir, None).unwrap(), "2020-01-01");
        fs::write(dir.join(TOOLCHAIN_FILE), "  \n").unwrap();
        assert!(resolve_version(&dir, None).is_err());
        fs::remove_file(dir.join(TOOLCHAIN_FILE)).unwrap();
        assert!(resolve_version(&dir, None).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
