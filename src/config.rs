use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use prettytable::{format, row, Table};
use serde_derive::{Deserialize, Serialize};

use crate::error::Result;
use crate::repo::Repo;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub repos: HashMap<String, Repo>,
}

impl Config {
    pub fn new(path: &Path) -> Result<Self> {
        // if the path doesn't exist, create the file first
        if !path.exists() {
            println!(
                "Config file not found at path \"{}\". Creating one.",
                path.to_str().unwrap_or("error displaying path")
            );
            let mut new_config = File::create(path)?;
            new_config.write_all(b"{\"repos\": {}}")?;
        }

        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        match serde_json::from_str(&content) {
            Ok(config) => Ok(config),
            Err(err) => Err(err.into()),
        }
    }

    pub fn repos_sorted(&self) -> Vec<(&String, &Repo)> {
        let mut ordered = self.repos.iter().collect::<Vec<_>>();

        ordered.sort_by(|a, b| a.0.cmp(b.0));

        ordered
    }

    pub fn list(&self) {
        let mut table = Table::new();
        let format = format::FormatBuilder::new().padding(1, 1).build();

        table.set_format(format);
        for (_, repo) in self.repos_sorted() {
            table.add_row(row![FW->&repo.key, &repo.path]);
        }

        table.print_tty(true).expect("Failed to render table");
    }

    pub fn add(&mut self, candidate: Repo) {
        self.repos.insert(candidate.key.clone(), candidate);
    }

    pub fn contains(&self, candidate: &Repo) -> bool {
        self.repos.values().any(|v| v == candidate)
    }

    pub fn remove(&mut self, key: &str) -> bool {
        self.repos.remove(key).is_some()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(&self)?;
        let mut file = File::create(path)?;
        file.write_all(&json)?;
        Ok(())
    }
}
