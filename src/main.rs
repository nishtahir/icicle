mod commands;
mod environment;
mod fs_util;
mod install;

use anyhow::Result;
use clap::{crate_description, crate_name, crate_version, Arg, Command};

fn version_arg(help: &'static str, required: bool) -> Arg {
    Arg::new("version").help(help).required(required).index(1)
}

fn cli() -> Command {
    Command::new(crate_name!())
        .version(crate_version!())
        .about(crate_description!())
        .arg_required_else_help(true)
        .subcommand(
            Command::new("use")
                .about("Change the oss cad toolchain version")
                .arg(version_arg(
                    "The version of the toolchain to use. Defaults to the '.icicle-toolchain' file",
                    false,
                )),
        )
        .subcommand(
            Command::new("default")
                .about("Set an installed toolchain as the default toolchain")
                .arg(version_arg(
                    "The version of the toolchain to set as default",
                    true,
                )),
        )
        .subcommand(
            Command::new("install")
                .about("Install an OSS CAD Suite toolchain")
                .arg(version_arg(
                    "The version of the toolchain to install. Defaults to the '.icicle-toolchain' file",
                    false,
                )),
        )
        .subcommand(Command::new("latest").about("Install the latest OSS CAD Suite toolchain"))
        .subcommand(
            Command::new("uninstall")
                .about("Uninstall an OSS CAD Suite toolchain")
                .arg(version_arg("The version of the toolchain to uninstall", true)),
        )
        .subcommand(Command::new("current").about("Print the active oss cad toolchain version"))
        .subcommand(
            Command::new("env").about("Print and setup required environment variables for icicle"),
        )
        .subcommand(Command::new("list").about("List installed toolchain versions"))
}

fn main() -> Result<()> {
    let matches = cli().get_matches();

    match matches.subcommand() {
        Some(("install", m)) => {
            commands::install(m.get_one::<String>("version").map(String::as_str))
        }
        Some(("latest", _)) => commands::latest(),
        Some(("uninstall", m)) => commands::uninstall(m.get_one::<String>("version").unwrap()),
        Some(("default", m)) => commands::set_default(m.get_one::<String>("version").unwrap()),
        Some(("use", m)) => {
            commands::use_toolchain(m.get_one::<String>("version").map(String::as_str))
        }
        Some(("current", _)) => commands::current(),
        Some(("env", _)) => commands::env_command(),
        Some(("list", _)) => commands::list(),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_is_valid() {
        super::cli().debug_assert();
    }
}
