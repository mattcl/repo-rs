use futures::stream::StreamExt;
use std::env;
use std::path::Path;
use std::process::Output;

use clap::ArgMatches;
use colored::*;
use rayon::prelude::*;

use config::Config;
use error::{RepoRsError, Result, UnwrapOrExit};
use repo::Repo;

mod cli;
mod config;
mod error;
mod github;
mod repo;

pub fn exit(message: &str) -> ! {
    let err = clap::Error::with_description(message, clap::ErrorKind::InvalidValue);
    err.exit();
}

fn pluralize_repos(config: &mut Config) -> String {
    let count = config.repos.len();
    let noun = match count {
        1 => "repo",
        _ => "repos",
    };
    format!("{} {}", count, noun)
}

fn collect_output(header: String, result: Output) -> Option<String> {
    if !result.stdout.is_empty() || !result.stderr.is_empty() {
        let mut output = header;

        if !result.stdout.is_empty() {
            output.push_str("\n");
            output.push_str(
                &String::from_utf8(result.stdout.clone()).expect("Output is not valid utf-8"),
            );
        }

        if !result.stderr.is_empty() {
            output.push_str("\n");
            output.push_str(
                &String::from_utf8(result.stderr.clone()).expect("Output is not valid utf-8"),
            );
        }

        return Some(output);
    }
    None
}

fn handle_errors(message: &str, results: Vec<Result<()>>) {
    let errors: Vec<Result<()>> = results.into_iter().filter(|r| r.is_err()).collect();

    if !errors.is_empty() {
        for e in errors {
            let error = e.err().unwrap();
            println!("{}", error);
        }
        exit(message);
    }
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
    println!("Attempting to update {}", pluralize_repos(config));

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
    println!(
        "Running `{}` in {}",
        raw_cmd.clone().join(" "),
        pluralize_repos(config)
    );

    let args: Vec<&str> = raw_cmd.drain(1..).collect();
    // this is safe, since we know we had at least one value
    let prog = raw_cmd.pop().unwrap();

    let results: Vec<Result<()>> = config
        .repos
        .par_iter_mut()
        .map(|(key, repo)| -> Result<()> {
            let result = repo.init().and_then(|_| repo.run(prog, args.clone()))?;
            let header = format!("{}", &key.green().bold());
            if let Some(output) = collect_output(header, result) {
                println!("{}", output);
            }
            Ok(())
        })
        .collect();

    handle_errors("Not all commands succeeded", results);

    println!("done")
}

fn status(config: &mut Config, all: bool) {
    println!("Getting status of {}", pluralize_repos(config));

    let results: Vec<Result<()>> = config
        .repos
        .par_iter_mut()
        .map(|(key, repo)| -> Result<()> {
            if let Some(result) = repo.init().and_then(|_| repo.status(!all))? {
                let header = format!("{}", &key.green().bold());
                if let Some(output) = collect_output(header, result) {
                    println!("{}", output);
                }
            }

            Ok(())
        })
        .collect();

    handle_errors("Could not get status of all repos", results);

    println!("done")
}

#[tokio::main]
async fn list_org_repos(org_name: String) {
    match github::org_repos(org_name).await {
        Ok(mut response) => {
            while let Some(repo_raw) = response.next().await {
                match repo_raw {
                    Ok(repo) => println!("{}", repo.full_name),
                    Err(err) => {
                        handle_errors(
                            "Could not fetch all repositories from the specified organization",
                            vec![Err(RepoRsError::from(err))],
                        );
                    }
                }
            }
        }
        Err(err) => {
            handle_errors(
                "Could not fetch repositories from the specified organization",
                vec![Err(err)],
            );
        }
    }
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

    match matches.subcommand() {
        ("list", Some(_)) => config.list(),
        ("track", Some(track_matches)) => track(&mut config, track_matches, config_file),
        ("untrack", Some(untrack_matches)) => untrack(&mut config, untrack_matches, config_file),
        ("pull", Some(pull_matches)) => {
            let allow_stash = pull_matches.is_present("stash");
            pull(&mut config, allow_stash)
        }
        ("run", Some(run_matches)) => {
            let mut raw_cmd: Vec<&str> = run_matches.values_of("cmd").unwrap().collect();
            run(&mut config, &mut raw_cmd)
        }
        ("status", Some(status_matches)) => {
            let all = status_matches.is_present("all");
            status(&mut config, all)
        }
        ("gh", Some(gh_matches)) => match gh_matches.subcommand() {
            ("list", Some(list_matches)) => {
                list_org_repos(list_matches.value_of("org").unwrap().to_string())
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}
