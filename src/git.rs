use crate::errors::*;
use crate::makepkg::{ResolvedPin, Source};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GitSource {
    pub url: String,
    pub commit: Option<String>,
    pub tag: Option<String>,
    pub signed: bool,
}

impl fmt::Display for GitSource {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        write!(w, "{}", self.url)?;
        if self.signed {
            write!(w, "?signed")?;
        }
        if let Some(commit) = &self.commit {
            write!(w, "#commit={}", commit)?;
        }
        if let Some(tag) = &self.tag {
            write!(w, "#tag={}", tag)?;
        }
        Ok(())
    }
}

impl FromStr for GitSource {
    type Err = Error;

    fn from_str(mut s: &str) -> Result<Self> {
        let mut signed = false;
        let mut commit = None;
        let mut tag = None;

        if let Some(remaining) = s.strip_suffix("?signed") {
            signed = true;
            s = remaining;
        }

        if let Some((remaining, value)) = s.rsplit_once("#commit=") {
            commit = Some(value.to_string());
            s = remaining;
        }

        if let Some((remaining, value)) = s.rsplit_once("#tag=") {
            tag = Some(value.to_string());
            s = remaining;
        }

        if let Some(remaining) = s.strip_suffix("?signed") {
            signed = true;
            s = remaining;
        }

        Ok(Self {
            url: s.to_string(),
            commit,
            tag,
            signed,
        })
    }
}

pub fn run(source: GitSource, repo_path: &Path) -> Result<ResolvedPin> {
    if !repo_path.exists() {
        bail!(
            "Repo does not exist yet, cloning is currently not supported {:?}",
            repo_path
        );
    }

    let repo = git_repository::open(repo_path).context("Failed to open repository")?;
    let tag_name = source.tag.as_ref().context("No tag configured")?;
    let tag_ref = format!("refs/tags/{}", tag_name);
    let tag = repo
        .find_reference(&tag_ref)
        .context("Failed to find tag")?;
    debug!("Resolved tag from repository: {:?}", tag);

    let tag_hash = tag
        .inner
        .target
        .try_into_id()
        .map_err(|r| anyhow!("Ref could not be turned into hash: {:?}", r))?
        .to_string();
    info!("Resolved tag {:?} to tag hash: {:?}", tag_name, tag_hash);
    let peeled = tag.inner.peeled.context("Failed to resolve tag")?;
    let commit_hash = peeled.to_string();
    info!(
        "Resolved tag {:?} to commit hash: {:?}",
        tag_name, commit_hash
    );
    Ok(ResolvedPin {
        tag_hash,
        commit_hash,
        source: Source::Git(source),
    })
}
