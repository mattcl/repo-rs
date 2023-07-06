use crate::error::{RepoRsError, Result};
use git2::{Repository, RepositoryState};
use serde_derive::{Deserialize, Serialize};
use std::path::Path;
use std::process::Output;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub key: String,
    pub path: String,
    pub remote: String,
    pub branch: String,
}

impl Repo {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(path: &str) -> RepoBuilder {
        RepoBuilder::new(path)
    }

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

    async fn stash(&self) -> Result<Output> {
        self.run("git", &["stash"]).await
    }

    async fn stash_pop(&self) -> Result<()> {
        self.run("git", &["stash", "pop"]).await?;
        Ok(())
    }

    fn current_branch(&self) -> Result<String> {
        let repo = self.repository()?;
        current_branch(&self.key, &repo)
    }

    async fn checkout(&self, branch: &str) -> Result<Output> {
        self.run("git", &["checkout", branch]).await
    }

    async fn rebase(&self) -> Result<Output> {
        self.run("git", &["pull", "--rebase"]).await
    }

    pub async fn run(&self, prog: &str, args: &[&str]) -> Result<Output> {
        let mut cmd = Command::new(prog);
        cmd.current_dir(&self.path).args(args);

        let result = cmd.output().await?;

        if !result.status.success() {
            return Err(RepoRsError::CommandFailed(self.key.clone(), cmd, result));
        }

        Ok(result)
    }

    pub async fn status(&self, require_dirty: bool) -> Result<Option<Output>> {
        if !require_dirty || self.is_dirty()? {
            return Ok(Some(self.run("git", &["status"]).await?));
        }
        Ok(None)
    }

    pub async fn update_repo(&self, allow_stash: bool) -> Result<Output> {
        // make sure there are no active merges/rebases/wahtever
        self.validate_working_state()?;

        // Find changes that would prevent us from rebasing or changing branches
        let dirty = self.is_dirty()?;

        // stash if necessary
        if dirty {
            if allow_stash {
                self.stash().await?;
            } else {
                return Err(RepoRsError::RepoDirty(self.key.clone()));
            }
        }

        // get current branch
        let original_branch = self.current_branch()?;
        let requires_branch_change = self.branch != original_branch;

        // switch to target branch if necessary
        if requires_branch_change {
            self.checkout(&self.branch).await?;
        }

        // pull and rebase
        let output = self.rebase().await?;

        // switch to the original branch if necessary
        if requires_branch_change {
            self.checkout(&original_branch).await?;
        }

        // undo the stash if necessary
        if dirty && allow_stash {
            self.stash_pop().await?;
        }

        Ok(output)
    }

    fn repository(&self) -> Result<Repository> {
        Ok(Repository::discover(&self.path)?)
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
    pub branch: Option<String>,
}

impl RepoBuilder {
    pub fn new(path: &str) -> RepoBuilder {
        RepoBuilder {
            key: None,
            path: path.to_owned(),
            remote: None,
            branch: None,
        }
    }

    pub fn key(&mut self, key: &str) {
        self.key = Some(key.to_owned());
    }

    pub fn remote(&mut self, remote: &str) {
        self.remote = Some(remote.to_owned());
    }

    pub fn branch(&mut self, branch: &str) {
        self.branch = Some(branch.to_owned());
    }

    pub fn build(mut self) -> Result<Repo> {
        let p = self.path.clone();
        let path = Path::new(&p);

        // attempt to instantiate the repository object
        let repository = Repository::discover(path)?;

        {
            let real_path = match repository.workdir() {
                Some(p) => p.to_str().unwrap().to_string(),
                None => return Err(RepoRsError::NoRepo(p)),
            };
            self.path = real_path;
        }

        let p = self.path.clone();
        let path = Path::new(&p);

        let key = match self.key {
            Some(k) => k,
            // attempt to derive key from repo path
            None => path.file_name().unwrap().to_str().unwrap().to_owned(),
        };

        let remote = match self.remote {
            Some(r) => r,
            None => {
                if let Some(candidate) = repository.remotes()?.get(0) {
                    candidate.to_owned()
                } else {
                    return Err(RepoRsError::NoRemotes(p));
                }
            }
        };

        let branch = match self.branch {
            Some(b) => b,
            // attempt to track the current branch if one was not specified
            None => current_branch(&key, &repository)?,
        };

        Ok(Repo {
            key,
            path: self.path.clone(),
            remote,
            branch,
        })
    }
}

// helper since we need to do this during the builder as well
fn current_branch(key: &str, repository: &Repository) -> Result<String> {
    let head = repository.head()?;
    match head.shorthand() {
        Some(b) => Ok(b.to_string()),
        None => Err(RepoRsError::BranchUnknown(key.to_string())),
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
        };

        let repo2 = Repo {
            key: "foo".to_string(),
            path: "hoof".to_string(),
            remote: "herp".to_string(),
            branch: "derp".to_string(),
        };

        let repo3 = Repo {
            key: "doof".to_string(),
            path: "bar".to_string(),
            remote: "herp1".to_string(),
            branch: "derp1".to_string(),
        };

        assert_eq!(true, repo1 == repo2);
        assert_eq!(true, repo1 == repo3);
        assert_eq!(false, repo2 == repo3);
    }
}
