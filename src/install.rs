use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::environment::{validate_version, Environment};
use crate::progress::ProgressReader;

const LATEST_RELEASE_URL: &str = "https://github.com/YosysHQ/oss-cad-suite-build/releases/latest";

pub fn download_url(env: &Environment, version: &str) -> String {
    format!(
        "{mirror}/{date}/oss-cad-suite-{os}-{arch}-{minified_date}.tgz",
        mirror = env.dist_mirror,
        date = version,
        os = env.os,
        arch = env.arch,
        minified_date = version.replace('-', "")
    )
}

/// Download and unpack a toolchain. Extraction happens in a staging directory that is
/// renamed into place once complete, so an interrupted install is never mistaken for a
/// finished one.
pub fn install(env: &Environment, version: &str) -> Result<()> {
    validate_version(version)?;

    let toolchain_path = env.home.toolchains.join(version);
    if toolchain_path.is_dir() {
        println!("'{}' is already installed.", version);
        return Ok(());
    }

    let url = download_url(env, version);
    println!("Downloading toolchain from {}", url);

    let response = match ureq::get(&url).call() {
        Ok(response) => response,
        Err(ureq::Error::Status(404, _)) => {
            bail!("Toolchain '{}' was not found at '{}'", version, url)
        }
        Err(e) => {
            return Err(e).with_context(|| format!("Failed to download toolchain from '{}'", url))
        }
    };

    let staging = env.home.toolchains.join(format!(".{}.partial", version));
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .with_context(|| format!("Failed to remove '{}'", staging.display()))?;
    }

    println!("Extracting toolchain to {}...", toolchain_path.display());
    let total = response
        .header("Content-Length")
        .and_then(|length| length.parse::<u64>().ok());
    let mut progress = ProgressReader::new(response.into_reader(), total);

    let result = unpack(&mut progress, &staging);
    progress.finish();

    let result = result.and_then(|_| {
        fs::rename(&staging, &toolchain_path)
            .with_context(|| format!("Failed to move toolchain to '{}'", toolchain_path.display()))
    });

    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn unpack(reader: impl std::io::Read, dest: &Path) -> Result<()> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(reader));
    archive
        .unpack(dest)
        .with_context(|| format!("Failed to unpack toolchain into '{}'", dest.display()))
}

/// GitHub redirects `releases/latest` to `releases/tag/<version>`
pub fn resolve_latest_version() -> Result<String> {
    let response = ureq::get(LATEST_RELEASE_URL).call().with_context(|| {
        format!(
            "Failed to resolve latest release from '{}'",
            LATEST_RELEASE_URL
        )
    })?;
    version_from_release_url(response.get_url())
}

pub fn version_from_release_url(url: &str) -> Result<String> {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|tag| *tag != "latest" && validate_version(tag).is_ok())
        .map(str::to_string)
        .with_context(|| format!("Failed to determine latest version from '{}'", url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_release_redirect() {
        let url = "https://github.com/YosysHQ/oss-cad-suite-build/releases/tag/2024-01-14";
        assert_eq!(version_from_release_url(url).unwrap(), "2024-01-14");
        assert_eq!(
            version_from_release_url(&format!("{url}/")).unwrap(),
            "2024-01-14"
        );
    }

    #[test]
    fn rejects_unresolved_latest() {
        assert!(version_from_release_url(LATEST_RELEASE_URL).is_err());
    }
}
