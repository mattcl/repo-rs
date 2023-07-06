use std::{env, fmt::Display};

use serde_json::error::Error as SerdeError;

/// RepoRsError enumerates all possible errors returned by this library
#[derive(Debug)]
pub enum RepoRsError {
    /// Represents a failure to determine the current branch
    BranchUnknown(String),

    /// Represents a command returning a nonzero exit code
    CommandFailed(String, tokio::process::Command, std::process::Output),

    /// Represents a repository with no remotes defined
    NoRemotes(String),

    /// Represents a path that is not within a valid non-empty git repository
    NoRepo(String),

    /// Represents repository that has local operations in progress
    OperationsInProgress(String),

    /// Represents repository that is dirty
    RepoDirty(String),

    /// Represents all other cases of `git2::Error`
    GitError(git2::Error),

    /// Represents all other cases of `github_v3::GHError`
    GithubError(github_v3::GHError),

    /// Represents all other cases of `std::io::Error`
    IOError(std::io::Error),

    /// Represents all other cases of `serde_json::error::Error`
    JsonError(SerdeError),

    /// Represents all other cases of `env::VarError`
    VarError(env::VarError),
}

impl std::error::Error for RepoRsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            RepoRsError::BranchUnknown(_) => None,
            RepoRsError::CommandFailed(_, _, _) => None,
            RepoRsError::NoRemotes(_) => None,
            RepoRsError::NoRepo(_) => None,
            RepoRsError::OperationsInProgress(_) => None,
            RepoRsError::RepoDirty(_) => None,
            RepoRsError::GitError(ref err) => Some(err),
            RepoRsError::GithubError(ref err) => Some(err),
            RepoRsError::IOError(ref err) => Some(err),
            RepoRsError::JsonError(ref err) => Some(err),
            RepoRsError::VarError(ref err) => Some(err),
        }
    }
}

impl std::fmt::Display for RepoRsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            RepoRsError::BranchUnknown(ref key) => {
                write!(f, "Could not determine current branch for '{}'", key)
            }
            RepoRsError::CommandFailed(ref key, ref command, ref output) => write!(
                f,
                "Error running `{:?}` in '{}': {:?}",
                command, key, output
            ),
            RepoRsError::NoRemotes(ref key) => write!(
                f,
                "No remotes found for '{}'. Please specify a remote for this repository",
                key
            ),
            RepoRsError::NoRepo(ref key) => write!(
                f,
                "Specified path '{}' is not at or in a valid, non-empty git repository",
                key
            ),
            RepoRsError::OperationsInProgress(ref key) => write!(
                f,
                "Repository '{}' has local git operations in progress",
                key
            ),
            RepoRsError::RepoDirty(ref key) => write!(
                f,
                "Repository '{}' is dirty. Maybe attempt with --stash option?",
                key
            ),
            RepoRsError::GitError(ref err) => err.fmt(f),
            RepoRsError::GithubError(ref err) => err.fmt(f),
            RepoRsError::IOError(ref err) => err.fmt(f),
            RepoRsError::JsonError(ref err) => err.fmt(f),
            RepoRsError::VarError(ref err) => err.fmt(f),
        }
    }
}

impl From<git2::Error> for RepoRsError {
    fn from(err: git2::Error) -> RepoRsError {
        RepoRsError::GitError(err)
    }
}

impl From<github_v3::GHError> for RepoRsError {
    fn from(err: github_v3::GHError) -> RepoRsError {
        RepoRsError::GithubError(err)
    }
}

impl From<std::io::Error> for RepoRsError {
    fn from(err: std::io::Error) -> RepoRsError {
        RepoRsError::IOError(err)
    }
}

impl From<SerdeError> for RepoRsError {
    fn from(err: SerdeError) -> RepoRsError {
        RepoRsError::JsonError(err)
    }
}

impl From<env::VarError> for RepoRsError {
    fn from(err: env::VarError) -> RepoRsError {
        RepoRsError::VarError(err)
    }
}

pub type Result<T> = std::result::Result<T, RepoRsError>;

pub trait UnwrapOrExit<T>
where
    Self: Sized,
{
    fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T;

    fn unwrap_or_exit(self, message: &str) -> T {
        let err = clap::Error::with_description(message, clap::ErrorKind::InvalidValue);
        self.unwrap_or_else(|| err.exit())
    }
}

impl<T> UnwrapOrExit<T> for Option<T> {
    fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.unwrap_or_else(f)
    }
}

impl<T, E> UnwrapOrExit<T> for std::result::Result<T, E>
where
    E: Display,
{
    fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.unwrap_or_else(|_| f())
    }

    fn unwrap_or_exit(self, message: &str) -> T {
        self.unwrap_or_else(|e| {
            let err = clap::Error::with_description(
                &format!("{}: {}", message, e),
                clap::ErrorKind::InvalidValue,
            );
            err.exit()
        })
    }
}
