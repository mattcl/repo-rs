use crate::error::{RepoRsError, Result};
use git2::{Repository, RepositoryState};
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Output};

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

    pub fn init(&mut self) -> Result<()> {
        self.repository = Some(Repository::discover(&self.path)?);
        Ok(())
    }

    // pub fn clone(url: &str, dest: &str, branch: &str) -> Result<Repo> {
    //     let repository = Repository::clone(url, dest)?;
    // }

    fn validate_working_state(&self) -> Result<()> {
        let repo = self.repository()?;

        match repo.state() {
            RepositoryState::Clean => Ok(()),
            _ => Err(RepoRsError::OperationsInProgress(self.key.clone())),
        }
    }

    fn is_dirty(&self) -> Result<bool> {
        let repo = self.repository()?;
        let index = repo.index()?;
        Ok(repo
            .diff_index_to_workdir(Some(&index), None)
            .map(|diff| diff.deltas().count() != 0)
            .unwrap_or(false))
    }

    fn stash(&self) -> Result<Output> {
        self.run("git", vec!["stash"])
    }

    fn stash_pop(&self) -> Result<()> {
        self.run("git", vec!["stash", "pop"])?;
        Ok(())
    }

    fn current_branch(&self) -> Result<String> {
        let repo = self.repository()?;
        let head = repo.head()?;
        match head.shorthand() {
            Some(b) => Ok(b.to_string()),
            None => Err(RepoRsError::BranchUnknown(self.key.clone())),
        }
    }

    fn checkout(&self, branch: &str) -> Result<Output> {
        self.run("git", vec!["checkout", branch])
    }

    fn rebase(&self) -> Result<Output> {
        self.run("git", vec!["pull", "--rebase"])
    }

    pub fn run(&self, prog: &str, args: Vec<&str>) -> Result<Output> {
        let mut cmd = Command::new(prog);
        cmd.current_dir(&self.path).args(args);

        let result = cmd.output()?;

        if !result.status.success() {
            return Err(RepoRsError::CommandFailed(self.key.clone(), cmd, result));
        }

        Ok(result)
    }

    pub fn status(&self, require_dirty: bool) -> Result<Option<Output>> {
        if !require_dirty || self.is_dirty()? {
            return Ok(Some(self.run("git", vec!["status"])?));
        }
        Ok(None)
    }

    pub fn update_repo(&self, allow_stash: bool) -> Result<Output> {
        // make sure there are no active merges/rebases/wahtever
        self.validate_working_state()?;

        // Find changes that would prevent us from rebasing or changing branches
        let dirty = self.is_dirty()?;

        // stash if necessary
        if dirty {
            if allow_stash {
                self.stash()?;
            } else {
                return Err(RepoRsError::RepoDirty(self.key.clone()));
            }
        }

        // get current branch
        let original_branch = self.current_branch()?;
        let requires_branch_change = self.branch != original_branch;

        // switch to target branch if necessary
        if requires_branch_change {
            self.checkout(&self.branch)?;
        }

        // pull and rebase
        let output = self.rebase()?;

        // switch to the original branch if necessary
        if requires_branch_change {
            self.checkout(&original_branch)?;
        }

        // undo the stash if necessary
        if dirty && allow_stash {
            self.stash_pop()?;
        }

        Ok(output)
    }

    fn repository<'a>(&'a self) -> Result<&'a Repository> {
        self.repository
            .as_ref()
            .ok_or(RepoRsError::UninitializedRepoObject(self.key.clone()))
    }
}

impl PartialEq for Repo {
    fn eq(&self, other: &Repo) -> bool {
        self.key == other.key || self.path == other.path
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
                None => return Err(RepoRsError::NoRepo(p)),
            };
            self.path = real_path;
        }

        let p = self.path.clone();
        let path = Path::new(&p);

        if self.key.is_none() {
            // attempt to derive key from repo path
            self.key = Some(path.file_name().unwrap().to_str().unwrap().to_owned());
        }

        if self.remote.is_none() {
            // use the first remote you can find
            if let Some(candidate) = repository.remotes()?.get(0) {
                self.remote = Some(candidate.to_owned());
            } else {
                return Err(RepoRsError::NoRemotes(p));
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

#[cfg(test)]
mod tests {
    use super::Repo;

    #[test]
    fn equality() {
        let repo1 = Repo {
            key: "foo".to_string(),
            path: "bar".to_string(),
            remote: "baz".to_string(),
            branch: "fez".to_string(),
            repository: None,
        };

        let repo2 = Repo {
            key: "foo".to_string(),
            path: "hoof".to_string(),
            remote: "herp".to_string(),
            branch: "derp".to_string(),
            repository: None,
        };

        let repo3 = Repo {
            key: "doof".to_string(),
            path: "bar".to_string(),
            remote: "herp1".to_string(),
            branch: "derp1".to_string(),
            repository: None,
        };

        assert_eq!(true, repo1 == repo2);
        assert_eq!(true, repo1 == repo3);
        assert_eq!(false, repo2 == repo3);
    }
}
