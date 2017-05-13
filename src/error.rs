use std::env;
use std::io;

use clap;
use git2;
use serde_json::error::Error as SerdeError;

error_chain! {
    foreign_links {
        VarError(env::VarError);
        IOError(io::Error);
        GitError(git2::Error);
        Json(SerdeError);
    }

    errors {
        RepoEmpty {
            description("repo is empty")
            display("repo is empty")
        }
    }
}

pub trait UnwrapOrExit<T>
    where Self: Sized
{
    fn unwrap_or_else<F>(self, f: F) -> T where F: FnOnce() -> T;

    fn unwrap_or_exit(self, message: &str) -> T {
        let err = clap::Error::with_description(message, clap::ErrorKind::InvalidValue);
        self.unwrap_or_else(|| err.exit())
    }
}

impl<T> UnwrapOrExit<T> for Option<T> {
    fn unwrap_or_else<F>(self, f: F) -> T
        where F: FnOnce() -> T
    {
        self.unwrap_or_else(f)
    }
}

impl<T> UnwrapOrExit<T> for Result<T> {
    fn unwrap_or_else<F>(self, f: F) -> T
        where F: FnOnce() -> T
    {
        self.unwrap_or_else(|_| f())
    }

    fn unwrap_or_exit(self, message: &str) -> T {
        self.unwrap_or_else(|e| {
            let err = clap::Error::with_description(&format!("{}: {}", message, e),
                                                    clap::ErrorKind::InvalidValue);
            err.exit()
        })
    }
}
