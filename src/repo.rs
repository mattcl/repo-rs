use error::{Error, ErrorKind, Result};
use git2::Repository;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct Repo {
    pub key: String,
    pub path: String,
    pub remote: String,
    pub branch: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub repository: Option<Repository>,
}

impl Repo {
    pub fn new(path: &str) -> RepoBuilder {
        RepoBuilder::new(path)
    }
}

pub struct RepoBuilder {
    pub key: Option<String>,
    pub path: String,
    pub remote: Option<String>,
    pub branch: String,
}

impl RepoBuilder {
    pub fn new(path: &str) -> Self {
        RepoBuilder {
            key: None,
            path: path.to_owned(),
            remote: None,
            branch: "master".to_owned(),
        }
    }

    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(key.to_owned());
        self
    }

    pub fn remote(mut self, remote: &str) -> Self {
        self.remote = Some(remote.to_owned());
        self
    }

    pub fn branch(mut self, branch: &str) -> Self {
        self.branch = branch.to_owned();
        self
    }

    pub fn build(mut self) -> Result<Repo> {
        let path = Path::new(&self.path);
        if self.key.is_none() {
            // attempt to derive key from repo path
            self.key = Some(path.file_name().unwrap().to_str().unwrap().to_owned());
        }

        // attempt to instantiate the repository object
        let repository = Repository::open(&path)?;

        if self.remote.is_none() {
            // use the first remote you can find
            if let Some(candidate) = repository.remotes()?.get(0) {
                self.remote = Some(candidate.to_owned());
            } else {
                return Err("No remotes found. Please specify remote for this repository".into());
            }
        }

        Ok(Repo {
            key: self.key.unwrap().clone(),
            path: self.path.clone(),
            remote: self.remote.unwrap().clone(),
            branch: self.branch.clone(),
            repository: Some(repository),
        })
    }
}
