use error::{Error, ErrorKind, Result};
use git2::Repository;
use std::path::Path;
use std::process::Command;

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

    // pub fn clone(url: &str, dest: &str, branch: &str) -> Result<Repo> {
    //     let repository = Repository::clone(url, dest)?;
    // }

    pub fn pull(&self) -> Result<()> {
        let repo = self.repository()?;

        // check if dirty

        // stash

        // switch to target branch

        // pull and rebase
        let result = Command::new("git")
            .current_dir(&self.path)
            .arg("pull")
            .arg("--rebase")
            .status()?;

        if !result.success() {
            return Err(format!("Error updating {}. Command exited with {}",
                               self.key,
                               result.code().unwrap())
                               .into());
        }

        // switch back

        Ok(())
    }

    fn repository<'a>(&'a self) -> Result<&'a Repository> {
        self.repository.as_ref().ok_or("Invalid git repo or repository not initialized".into())
    }
}

pub struct RepoBuilder {
    pub key: Option<String>,
    pub path: String,
    pub remote: Option<String>,
    pub branch: String,
}

impl RepoBuilder {
    pub fn new(path: &str) -> RepoBuilder {
        RepoBuilder {
            key: None,
            path: path.to_owned(),
            remote: None,
            branch: "master".to_owned(),
        }
    }

    pub fn key(&mut self, key: &str) {
        self.key = Some(key.to_owned());
    }

    pub fn remote(&mut self, remote: &str) {
        self.remote = Some(remote.to_owned());
    }

    pub fn branch(&mut self, branch: &str) {
        self.branch = branch.to_owned();
    }

    pub fn build(mut self) -> Result<Repo> {
        let p = self.path.clone();
        let path = Path::new(&p);

        // attempt to instantiate the repository object
        let repository = Repository::discover(&path)?;

        {
            let real_path = match repository.workdir() {
                Some(p) => p.to_str().unwrap().to_string(),
                None => {
                    return Err("Path does not exist at or within a valid, non-empty git repo"
                                   .into())
                }
            };
            self.path = real_path;
        }


        if self.key.is_none() {
            // attempt to derive key from repo path
            self.key = Some(path.file_name().unwrap().to_str().unwrap().to_owned());
        }


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
