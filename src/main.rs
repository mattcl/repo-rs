#[macro_use]
extern crate clap;
extern crate colored;
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

use colored::*;
use config::Config;
use error::{Result, UnwrapOrExit};
use rayon::prelude::*;
use repo::Repo;

mod cli;
mod config;
mod repo;
mod error;

pub fn exit(message: &str) -> ! {
    let err = clap::Error::with_description(message, clap::ErrorKind::InvalidValue);
    err.exit();
}

fn main() {
    let default_config_path_raw = env::home_dir()
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
                println!("Tracking branch '{}' from remote '{}' of '{}' at '{}'",
                         &repo.branch.white().bold(),
                         &repo.remote.white().bold(),
                         &repo.key.white().bold(),
                         &repo.path.white().bold());
                config.add(repo);
                config
                    .save(&config_file)
                    .unwrap_or_exit("Error saving config");
            }
        }
        Some("untrack") => {
            // The following two lines are safe because of the way clap validates params
            let subcmd = matches.subcommand_matches("untrack").unwrap();
            let key = subcmd.value_of("key").unwrap();
            if config.remove(&key) {
                println!("Stopped tracking {}", key.white().bold());
                config
                    .save(&config_file)
                    .unwrap_or_exit("Error saving config");
            }

        }
        Some("pull") => {
            let subcommand = matches.subcommand_matches("pull").unwrap();
            let allow_stash = subcommand.is_present("stash");
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
                         repo.init().and_then(|_| repo.update_repo(allow_stash))
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
        _ => exit("not implemented"), // TODO: change to unreachable!()
    };
}
