mod commands;
mod environment;
mod fs_util;
mod install;
mod progress;

use anyhow::Result;
use clap::{crate_description, crate_name, crate_version, value_parser, Arg, ArgAction, Command};
use clap_complete::Shell;

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
        .subcommand(
            Command::new("exec")
                .about("Run a command with a toolchain's tools, without changing the shell")
                .arg(
                    Arg::new("using")
                        .long("using")
                        .value_name("VERSION")
                        .help("The toolchain to run with. Defaults to the '.icicle-toolchain' file, then the default"),
                )
                .arg(
                    Arg::new("command")
                        .help("The command to run and its arguments")
                        .required(true)
                        .num_args(1..)
                        .trailing_var_arg(true)
                        .allow_hyphen_values(true),
                ),
        )
        .subcommand(
            Command::new("which")
                .about("Print the path of a tool in the active toolchain")
                .arg(
                    Arg::new("using")
                        .long("using")
                        .value_name("VERSION")
                        .help("Look in this toolchain instead of the one this shell is using"),
                )
                .arg(Arg::new("tool").help("The tool to look for").required(true)),
        )
        .subcommand(
            Command::new("prune")
                .about("Remove toolchains other than the default and those in use")
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .short('n')
                        .action(ArgAction::SetTrue)
                        .help("Show what would be removed without removing it"),
                )
                .arg(
                    Arg::new("yes")
                        .long("yes")
                        .short('y')
                        .action(ArgAction::SetTrue)
                        .help("Don't ask for confirmation"),
                ),
        )
        .subcommand(
            Command::new("completions")
                .about("Print a shell completion script")
                .arg(
                    Arg::new("shell")
                        .help("The shell to generate completions for")
                        .required(true)
                        .value_parser(value_parser!(Shell)),
                ),
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
        Some(("exec", m)) => {
            let command: Vec<String> = m.get_many::<String>("command").unwrap().cloned().collect();
            commands::exec(m.get_one::<String>("using").map(String::as_str), &command)
        }
        Some(("which", m)) => commands::which(
            m.get_one::<String>("tool").unwrap(),
            m.get_one::<String>("using").map(String::as_str),
        ),
        Some(("prune", m)) => commands::prune(m.get_flag("dry-run"), m.get_flag("yes")),
        Some(("completions", m)) => {
            let shell = *m.get_one::<Shell>("shell").unwrap();
            clap_complete::generate(shell, &mut cli(), crate_name!(), &mut std::io::stdout());
            Ok(())
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
