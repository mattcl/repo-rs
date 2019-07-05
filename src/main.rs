#[macro_use]
extern crate clap;
extern crate colored;
extern crate dirs;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate git2;
#[macro_use]
extern crate prettytable;
extern crate rayon;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;

use std::env;
use std::path::Path;

use clap::ArgMatches;
use colored::*;
use config::Config;
use error::{Result, UnwrapOrExit};
use rayon::prelude::*;
use repo::Repo;

mod cli;
mod config;
mod error;
mod repo;

pub fn exit(message: &str) -> ! {
    let err = clap::Error::with_description(message, clap::ErrorKind::InvalidValue);
    err.exit();
}

fn track(config: &mut Config, subcmd: &ArgMatches, config_file: &Path) {
    // this has a default value, so safe to unwrap
    let repo_path = subcmd.value_of("path").unwrap();

    let mut builder = Repo::new(repo_path);

    if let Some(key) = subcmd.value_of("key") {
        builder.key(key);
    }

    if let Some(remote) = subcmd.value_of("remote") {
        builder.remote(remote);
    }

    // this is safe because we have a default value
    let branch = subcmd.value_of("branch").unwrap();
    builder.branch(branch);
    let repo = builder.build().unwrap_or_exit("Error tracking repository");

    if config.contains(&repo) {
        exit("Repo is already being tracked")
    } else {
        println!(
            "Tracking branch '{}' from remote '{}' of '{}' at '{}'",
            &repo.branch.white().bold(),
            &repo.remote.white().bold(),
            &repo.key.white().bold(),
            &repo.path.white().bold()
        );
        config.add(repo);
        config
            .save(config_file)
            .unwrap_or_exit("Error saving config");
    }
}

fn untrack(config: &mut Config, subcmd: &ArgMatches, config_file: &Path) {
    // The following two lines are safe because of the way clap validates params
    let key = subcmd.value_of("key").unwrap();
    if config.remove(&key) {
        println!("Stopped tracking {}", key.white().bold());
        config
            .save(config_file)
            .unwrap_or_exit("Error saving config");
    }
}

fn pull(config: &mut Config, allow_stash: bool) {
    let count = config.repos.len();
    let noun = match count {
        1 => "repo",
        _ => "repos",
    };

    println!("Attempting to update {} {}", count, noun);

    let errors: Vec<Result<()>> = config
        .repos
        .par_iter_mut()
        .map(|(key, repo)| -> Result<()> {
            let s = format!("Updating {}", &key.white().bold());
            println!("{}", s);
            repo.init().and_then(|_| repo.update_repo(allow_stash))?;
            Ok(())
        })
        .filter(|r| r.is_err())
        .collect();

    if !errors.is_empty() {
        for e in errors {
            let error = e.err().unwrap();
            println!("{}", error);
        }
        exit("Not all repos could be updated");
    }
}

fn run(config: &mut Config, raw_cmd: &mut Vec<&str>) {
    let count = config.repos.len();
    let noun = match count {
        1 => "repo",
        _ => "repos",
    };

    println!(
        "Running `{}` in {} {}",
        raw_cmd.clone().join(" "),
        count,
        noun
    );

    let args: Vec<&str> = raw_cmd.drain(1..).collect();
    // this is safe, since we know we had at least one value
    let prog = raw_cmd.pop().unwrap();

    let output: Vec<Result<()>> = config
        .repos
        .par_iter_mut()
        .map(|(key, repo)| -> Result<()> {
            let result = repo.init().and_then(|_| repo.run(prog, args.clone()))?;
            if !result.stdout.is_empty() || !result.stderr.is_empty() {
                let mut output = format!("{}", &key.green().bold());

                if !result.stdout.is_empty() {
                    output.push_str("\n");
                    output.push_str(
                        &String::from_utf8(result.stdout)
                            .expect("Output is not valid utf-8"),
                    );
                }

                if !result.stderr.is_empty() {
                    output.push_str("\n");
                    output.push_str(
                        &String::from_utf8(result.stderr)
                            .expect("Output is not valid utf-8"),
                    );
                }

                println!("{}", output);
            }
            Ok(())
        })
        .collect();

    let errors: Vec<Result<()>> = output.into_iter().filter(|r| r.is_err()).collect();

    if !errors.is_empty() {
        for e in errors {
            let error = e.err().unwrap();
            println!("{}", error);
        }
        exit("Not all commands succeeded");
    }

    println!("done")
}

fn status(config: &mut Config, all: bool) {
    let count = config.repos.len();
    let noun = match count {
        1 => "repo",
        _ => "repos",
    };

    println!("Getting status of {} {}", count, noun);

    let output: Vec<Result<()>> = config
        .repos
        .par_iter_mut()
        .map(|(key, repo)| -> Result<()> {
            if let Some(result) = repo.init().and_then(|_| repo.status(!all))? {
                if !result.stdout.is_empty() || !result.stderr.is_empty() {
                    let mut output = format!("{}", &key.green().bold());

                    if !result.stdout.is_empty() {
                        output.push_str("\n");
                        output.push_str(
                            &String::from_utf8(result.stdout)
                                .expect("Output is not valid utf-8"),
                        );
                    }

                    if !result.stderr.is_empty() {
                        output.push_str("\n");
                        output.push_str(
                            &String::from_utf8(result.stderr)
                                .expect("Output is not valid utf-8"),
                        );
                    }

                    println!("{}", output);
                }
            }

            Ok(())
        })
        .collect();

    let errors: Vec<Result<()>> = output.into_iter().filter(|r| r.is_err()).collect();

    if !errors.is_empty() {
        for e in errors {
            let error = e.err().unwrap();
            println!("{}", error);
        }
        exit("Unable to get status from all repos");
    }

    println!("done")
}

fn main() {
    let default_config_path_raw = dirs::home_dir()
        .expect("could not determine home directory")
        .join(".repo-rs.json");
    let default_config_path = default_config_path_raw.to_str().unwrap();
    let default_track_path_raw = env::current_dir().expect("could not determine current directory");
    let default_track_path = default_track_path_raw.to_str().unwrap();
    let matches = cli::get_matches(default_config_path, default_track_path);

    let config_file = Path::new(matches.value_of("config").unwrap());
    let mut config = Config::new(&config_file).unwrap_or_exit("Error loading config");

    match matches.subcommand_name() {
        Some("list") => config.list(),
        Some("track") => {
            // we've checked already, so safe to unwrap
            let subcmd = matches.subcommand_matches("track").unwrap();
            track(&mut config, subcmd, config_file)
        }
        Some("untrack") => {
            let subcmd = matches.subcommand_matches("untrack").unwrap();
            untrack(&mut config, subcmd, config_file)
        }
        Some("pull") => {
            let subcmd = matches.subcommand_matches("pull").unwrap();
            let allow_stash = subcmd.is_present("stash");
            pull(&mut config, allow_stash)
        }
        Some("run") => {
            let subcmd = matches.subcommand_matches("run").unwrap();
            let mut raw_cmd: Vec<&str> = subcmd.values_of("cmd").unwrap().collect();
            run(&mut config, &mut raw_cmd)
        }
        Some("status") => {
            let subcmd = matches.subcommand_matches("status").unwrap();
            let all = subcmd.is_present("all");
            status(&mut config, all)
        }
        _ => unreachable!(),
    };
}
