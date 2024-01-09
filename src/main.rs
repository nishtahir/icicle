use std::env::{self, temp_dir};
use std::ffi::OsStr;
use std::fs::{self};
use std::io::BufReader;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{crate_description, crate_name, crate_version, Arg, Command};
use uuid::Uuid;

struct Environment {
    icicle_home: PathBuf,
    aliases_home: PathBuf,
    toolchain_home: PathBuf,
    caches_home: PathBuf,
    shell_path: PathBuf,
    os: String,
    arch: String,
    toolchain_file: String,
}

impl Environment {
    fn create() -> Result<Environment> {
        let icicle_home = env::var("ICICLE_HOME").with_context(|| {
            "'ICICLE_HOME' environment variable not set. Did you setup your shell to 'eval $(icicle env)'?"
        })?;

        let shell_path = env::var("ICICLE_SHELL_PATH").with_context(|| {
            "'ICICLE_SHELL_PATH' environment variable not set. Did you setup your shell to 'eval $(icicle env)'?"
        })?;

        let os = match env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            _ => bail!("Unsupported OS '{}'", env::consts::OS),
        };

        let arch = match env::consts::ARCH {
            "x86_64" | "amd64" => "x64",
            "aarch64" => "arm64",
            _ => bail!("Unsupported architecture '{}'", env::consts::ARCH),
        };

        let aliases_home = Path::new(&icicle_home).join("aliases");
        let toolchain_home = Path::new(&icicle_home).join("toolchains");
        let caches_home = Path::new(&icicle_home).join("caches");

        // make the dirs if they don't exist
        // this is really just for convenience so we can avoid checks later

        fs::create_dir_all(&icicle_home)
            .with_context(|| format!("Failed to create icicle home directory '{}'", icicle_home))?;

        fs::create_dir_all(&aliases_home).with_context(|| {
            format!(
                "Failed to create aliases home directory '{}'",
                aliases_home.display()
            )
        })?;

        fs::create_dir_all(&toolchain_home).with_context(|| {
            format!(
                "Failed to create toolchain home directory '{}'",
                toolchain_home.display()
            )
        })?;

        fs::create_dir_all(&caches_home).with_context(|| {
            format!(
                "Failed to create caches home directory '{}'",
                caches_home.display()
            )
        })?;

        Ok(Environment {
            icicle_home: Path::new(&icicle_home).to_path_buf(),
            aliases_home,
            toolchain_home,
            caches_home,
            shell_path: Path::new(&shell_path).to_path_buf(),
            os: os.to_string(),
            arch: arch.to_string(),
            toolchain_file: ".icicle-toolchain".to_string(),
        })
    }
}

fn main() -> Result<()> {
    let matches = Command::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg_required_else_help(true)
        .subcommand(
            Command::new("use")
                .about("Change the oss cad toolchain version")
                .arg(
                    Arg::new("version")
                        .help("The version of the toolchain to use")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("default")
                .about("Set an installed toolchain as the default toolchain")
                .arg(
                    Arg::new("version")
                        .help("The version of the toolchain to set as default")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("install")
                .about("Install an OSS CAD Suite toolchain")
                .arg(
                    Arg::new("version")
                        .help("The version of the toolchain to install")
                        .required(false)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("uninstall")
                .about("Uninstall an OSS CAD Suite toolchain")
                .arg(
                    Arg::new("version")
                        .help("The version of the toolchain to install")
                        .required(false)
                        .index(1),
                ),
        )
        .subcommand(Command::new("current").about("Print the active oss cad toolchain version"))
        .subcommand(
            Command::new("env").about("Print and setup required environment variables for icicle"),
        )
        .subcommand(Command::new("list").about("List installed toolchain versions"))
        .get_matches();

    // You can check what subcommand was used like so:
    match matches.subcommand() {
        Some(("install", sub_matches)) => {
            let version = sub_matches.get_one::<String>("version");
            handle_install(version)?;
        }
        Some(("uninstall", sub_matches)) => {
            let version: &String = sub_matches.get_one::<String>("version").unwrap();
            handle_uninstall(&version)?;
        }
        Some(("default", sub_matches)) => {
            let version: &String = sub_matches.get_one::<String>("version").unwrap();
            handle_default(&version)?;
        }
        Some(("use", sub_matches)) => {
            // it's required, so safe to call unwrap()
            let version = sub_matches.get_one::<String>("version");
            handle_use(version)?;
        }
        Some(("current", _)) => handle_current()?,
        Some(("env", _)) => handle_env()?,
        Some(("list", _)) => handle_list()?,
        _ => unreachable!(),
    }

    Ok(())
}

fn handle_install(version: Option<&String>) -> Result<()> {
    let env = Environment::create()?;
    let version = get_version(&env, version)?;

    let toolchain_path = Path::new(&env.toolchain_home).join(&version);
    if toolchain_path.is_dir() {
        println!("'{}' is already installed.", &version);
        return Ok(());
    }

    let target_url = format!(
        "https://github.com/YosysHQ/oss-cad-suite-build/releases/download/{date}/oss-cad-suite-{os}-{arch}-{minified_date}.tgz",
        date = version,
        os = env.os,
        arch = env.arch,
        minified_date = version.replace("-", "")
    );

    println!("Downloading toolchain from {}", target_url);

    let path = Path::new(&target_url);
    let file_name_without_ext = path
        .file_stem()
        .and_then(OsStr::to_str)
        .with_context(|| format!("Failed to extract file name from url '{}'", target_url))?;

    let temp_file = temp_dir()
        .join("icicle")
        .join(file_name_without_ext)
        .with_extension("tgz");

    let response = ureq::get(&target_url)
        .call()
        .with_context(|| format!("Failed to download toolchain from '{}'", target_url))?
        .into_reader();

    fs::create_dir_all(&temp_file.parent().unwrap()).with_context(|| {
        format!(
            "Failed to create temporary directory '{}'",
            temp_file.display()
        )
    })?;

    let mut file = fs::File::create(&temp_file)
        .with_context(|| format!("Failed to create temporary file '{}'", temp_file.display()))?;

    std::io::copy(&mut BufReader::new(response), &mut file).with_context(|| {
        format!(
            "Failed to write to temporary file '{}'",
            temp_file.display()
        )
    })?;

    // Extract the downloaded tarball
    println!(
        "Extracting toolchain to {}... ",
        env.toolchain_home.display()
    );
    let tar_gz = fs::File::open(&temp_file)
        .with_context(|| format!("Failed to open temporary file '{}'", temp_file.display()))?;

    let tar = flate2::read::GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(tar);

    archive
        .unpack(&env.toolchain_home.join(version))
        .with_context(|| format!("Failed to unpack tarball '{}'", temp_file.display()))?;

    println!("Cleaning up...");
    fs::remove_file(&temp_file)
        .with_context(|| format!("Failed to remove temporary file '{}'", temp_file.display()))?;

    Ok(())
}

fn get_version(env: &Environment, version: Option<&String>) -> Result<String> {
    let cwd = env::current_dir().with_context(|| "Failed to get current directory")?;
    let toolchain_file = Path::new(&cwd).join(&env.toolchain_file);

    let mut toolchain_file_version = None;
    if toolchain_file.exists() {
        toolchain_file_version = fs::read_to_string(&toolchain_file)
            .map(|content| Some(content.trim().to_string()))
            .with_context(|| format!("Failed to read file '{}'", toolchain_file.display()))?;
    }

    let version = version.or(toolchain_file_version.as_ref()).ok_or_else(|| {
        anyhow::anyhow!(
            "No version specified. Please specify a version or create a \
            '{}' file with the version you want to use.",
            env.toolchain_file
        )
    })?;

    Ok(version.to_owned())
}

fn handle_uninstall(version: &str) -> Result<()> {
    let env = Environment::create()?;

    let toolchain_path = Path::new(&env.toolchain_home).join(version);
    if !toolchain_path.is_dir() {
        bail!("'{}' is not a valid toolchain directory.", version);
    }

    fs::remove_dir_all(&toolchain_path).with_context(|| {
        format!(
            "Failed to remove toolchain directory '{}'",
            toolchain_path.display()
        )
    })?;

    Ok(())
}

fn handle_default(version: &str) -> Result<()> {
    let env = Environment::create()?;
    let aliases_home = Path::new(&env.aliases_home);
    fs::create_dir_all(&aliases_home).expect("Failed to create aliases directory");

    let toolchain_path = Path::new(&env.toolchain_home).join(version);
    if !toolchain_path.is_dir() {
        bail!("'{}' is not a valid toolchain directory.", version);
    }

    let default_alias = aliases_home.join("default");
    if default_alias.exists() {
        fs::remove_file(&default_alias).with_context(|| "Failed to remove existing alias")?;
    }

    println!("Setting {} as default.", version);
    unix_fs::symlink(&toolchain_path, &default_alias)
        .with_context(|| "Failed to create symbolic link")?;

    Ok(())
}

fn handle_env() -> Result<()> {
    // The user is trying to configure their shell to use icicle
    // We don't expect them to have ICICLE_HOME set yet

    // If the user has a home set use that otherwise default to ~/.icicle
    let icicle_home = env::var("ICICLE_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME").expect("Failed to get home directory");
        Path::new(&home)
            .join(".icicle")
            .to_string_lossy()
            .to_string()
    });

    let aliases_home = Path::new(&icicle_home).join("aliases");
    let caches_home = Path::new(&icicle_home).join("caches");
    let default_alias = Path::new(&aliases_home).join("default");

    fs::create_dir_all(&caches_home).with_context(|| "Failed to create caches directory")?;

    let file_name = format!("icicle_{}", Uuid::new_v4());
    let shell_session_sym_link = &caches_home.join(file_name);

    if shell_session_sym_link.exists() {
        fs::remove_file(&shell_session_sym_link)
            .with_context(|| "Failed to remove existing symlink")?;
    }

    unix_fs::symlink(default_alias, &shell_session_sym_link)
        .with_context(|| "Failed to create symlink")?;

    let bin_path = shell_session_sym_link.join("oss-cad-suite/bin");
    let lib_exec_path = shell_session_sym_link.join("oss-cad-suite/libexec");

    println!(
        "export ICICLE_SHELL_PATH={}",
        shell_session_sym_link.display()
    );
    println!("export ICICLE_HOME={}", icicle_home);
    println!("export PATH={}:$PATH", bin_path.display());
    println!("export PATH={}:$PATH", lib_exec_path.display());

    Ok(())
}

fn handle_current() -> Result<()> {
    let env = Environment::create()?;

    let shell_symlink = fs::canonicalize(env.shell_path)
        .with_context(|| "Failed to resolve toolchain directory")?;

    let version_folder = shell_symlink
        .to_str()
        .unwrap()
        .trim_start_matches(&format!("{}/", env.toolchain_home.to_string_lossy()));

    println!("{}", version_folder);
    Ok(())
}

fn handle_use(version: Option<&String>) -> Result<()> {
    let env = Environment::create()?;
    let version = get_version(&env, version)?;

    let search_paths = vec![
        Path::new(&env.toolchain_home).join(&version),
        Path::new(&env.aliases_home).join(&version),
    ];

    let toolchain = search_paths
        .into_iter()
        .find(|path| path.exists())
        .with_context(|| "Unable to find toolchain. Did you install it with 'icicle install'?")?;

    if !toolchain.is_dir() {
        bail!("'{}' is not a valid toolchain directory.", version);
    }

    let shell_symlink = Path::new(&env.shell_path);
    if shell_symlink.exists() {
        fs::remove_file(shell_symlink).with_context(|| "Failed to remove existing symlink")?;
    }

    // TODO - handle windows
    unix_fs::symlink(&toolchain, shell_symlink).with_context(|| "Failed to create symlink")?;

    Ok(())
}

fn handle_list() -> Result<()> {
    let env = Environment::create()?;

    let toolchain_home: PathBuf = Path::new(&env.icicle_home).join("toolchains");
    let aliases_home = Path::new(&env.icicle_home).join("aliases");

    // Resolve default alias
    let default_alias = Path::new(&aliases_home).join("default");

    // The user might not have a default set
    let mut default_alias_path = None;
    if default_alias.exists() {
        default_alias_path = Some(fs::canonicalize(&default_alias).with_context(|| {
            format!(
                "Failed to resolve default alias '{}'",
                default_alias.display()
            )
        })?);
    }

    let entries = fs::read_dir(&toolchain_home).with_context(|| {
        format!(
            "Failed to read toolchain directory '{}'",
            toolchain_home.display()
        )
    })?;

    for entry in entries {
        if let Ok(entry) = entry {
            if entry.path().is_dir() {
                let mut line: String = format!("* {}", entry.file_name().to_string_lossy());
                if Some(fs::canonicalize(entry.path()).unwrap()) == default_alias_path {
                    line.push_str(" (default)");
                }
                println!("{}", line);
            }
        }
    }

    Ok(())
}
