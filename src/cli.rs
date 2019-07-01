use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

pub fn get_matches<'a>(
    default_config_path: &'a str,
    default_track_path: &'a str,
) -> ArgMatches<'a> {
    let app = App::new("repo-rs")
        .about("Manage multiple git repositories")
        .author(crate_authors!())
        .version(crate_version!())
        .global_setting(AppSettings::ColorAuto)
        .global_setting(AppSettings::ColoredHelp)
        .arg(
            Arg::with_name("config")
                .help("sets the config file to use")
                .takes_value(true)
                .default_value(default_config_path)
                .short("c")
                .long("config")
                .global(true),
        )
        .subcommand(SubCommand::with_name("list").about("lists tracked repos"))
        .subcommand(SubCommand::with_name("status").about("gets status for tracked repos"))
        .subcommand(
            SubCommand::with_name("track")
                .about("track an existing repo")
                .arg(
                    Arg::with_name("path")
                        .help(
                            "The path of the repository to track. If the path is not \
                             the repository root, we will attempt to discover the root.",
                        )
                        .index(1)
                        .default_value(default_track_path)
                        .required(false),
                )
                .arg(
                    Arg::with_name("key")
                        .help(
                            "A unique identifier for the tracked repo (will use the \
                             repo directory name if not specified)",
                        )
                        .takes_value(true)
                        .required(false)
                        .short("k")
                        .long("key"),
                )
                .arg(
                    Arg::with_name("branch")
                        .help("The branch to track")
                        .takes_value(true)
                        .default_value("master")
                        .short("b")
                        .long("branch"),
                )
                .arg(
                    Arg::with_name("remote")
                        .help(
                            "The remote to sync with (will use the first listed remote \
                             if not specified).",
                        )
                        .takes_value(true)
                        .required(false)
                        .short("r")
                        .long("remote"),
                ),
        )
        .subcommand(
            SubCommand::with_name("untrack")
                .about("stop tracking a repo")
                .arg(
                    Arg::with_name("key")
                        .help("The key of the repo to untrack.")
                        .index(1)
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("clone")
                .about("clone a repo and start tracking it")
                .arg(
                    Arg::with_name("url")
                        .help("The clone url of the repository to track.")
                        .index(1)
                        .required(true),
                )
                .arg(
                    Arg::with_name("key")
                        .help(
                            "A unique identifier for the tracked repo (will use the \
                             repo directory name if not specified).",
                        )
                        .takes_value(true)
                        .required(false)
                        .short("k")
                        .long("key"),
                )
                .arg(
                    Arg::with_name("branch")
                        .help("The branch to track")
                        .takes_value(true)
                        .default_value("master")
                        .short("b")
                        .long("branch"),
                ),
        )
        .subcommand(SubCommand::with_name("prune").about("prune merged/deleted branches"))
        .subcommand(
            SubCommand::with_name("pull")
                .about("pull all tracked repos")
                .arg(
                    Arg::with_name("stash")
                        .help(
                            "Stash any uncommitted changes prior to the merge. \
                             If not specified, repos containing un-stashed changes \
                             will be skipped.",
                        )
                        .short("s")
                        .long("stash"),
                ),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("run a command in all tracked repos")
                .arg(
                    Arg::with_name("cmd")
                        .multiple(true)
                        .allow_hyphen_values(true)
                        .required(true)
                        .last(true),
                ),
        );
    app.get_matches()
}
