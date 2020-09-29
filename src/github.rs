use futures::Stream;
use github_v3::{Client, GHError};
use serde_derive::*;

use crate::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRepo {
    pub id: u32,
    pub name: String,
    pub full_name: String,
    pub ssh_url: String,
}

pub async fn org_repos(
    org: String,
) -> error::Result<impl Stream<Item = Result<RemoteRepo, GHError>>> {
    let gh = Client::new_from_env();
    Ok(gh
        .get()
        .path("orgs")
        .arg(&org)
        .arg("repos")
        .send()
        .await?
        .array::<RemoteRepo>())
}
