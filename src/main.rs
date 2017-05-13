#[macro_use]
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate futures;
extern crate git2;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tokio_core;

use std::env;
use std::path::Path;

use config::Config;
use error::UnwrapOrExit;

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
    let matches = cli::get_matches(default_config_path);

    let config_file = Path::new(matches.value_of("config").unwrap());
    let config = Config::new(&config_file).unwrap_or_exit("Error loading config");

    let res = match matches.subcommand_name() {
        Some("list") => config.list(),
        Some("track") => config.list(),
        _ => exit("not implemented"), // TODO: change to unreachable!()
    };
}
