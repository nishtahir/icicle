use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::Path;

use anyhow::{Context, Result};
use uuid::Uuid;

/// Point the symlink at `link` to `target`, replacing whatever symlink is already there.
///
/// This never consults `Path::exists`, which follows symlinks and reports a dangling link
/// as missing. The new link is created beside the old one and renamed over it, so a shell
/// reading the link never observes it missing.
pub fn replace_symlink(target: &Path, link: &Path) -> Result<()> {
    let parent = link
        .parent()
        .context("Symlink path has no parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create directory '{}'", parent.display()))?;

    let staging = parent.join(format!(".icicle-link-{}", Uuid::new_v4()));
    unix_fs::symlink(target, &staging)
        .with_context(|| format!("Failed to create symlink '{}'", staging.display()))?;

    fs::rename(&staging, link).map_err(|e| {
        let _ = fs::remove_file(&staging);
        anyhow::Error::new(e).context(format!("Failed to create symlink '{}'", link.display()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("icicle-ut-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn replaces_dangling_symlink() {
        let dir = scratch("dangling");
        let link = dir.join("link");
        unix_fs::symlink(dir.join("missing"), &link).unwrap();
        assert!(!link.exists()); // dangling

        let target = dir.join("target");
        fs::create_dir(&target).unwrap();
        replace_symlink(&target, &link).unwrap();
        assert_eq!(fs::read_link(&link).unwrap(), target);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn creates_missing_link_and_parent() {
        let dir = scratch("missing");
        let link = dir.join("nested/link");
        replace_symlink(&dir, &link).unwrap();
        assert_eq!(fs::read_link(&link).unwrap(), dir);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn leaves_no_staging_files() {
        let dir = scratch("staging");
        let link = dir.join("link");
        replace_symlink(&dir, &link).unwrap();
        replace_symlink(&dir, &link).unwrap();
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}
