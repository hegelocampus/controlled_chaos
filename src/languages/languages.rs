mod javascript;

use anyhow::{anyhow, Result};
use git2::Repository;
use javascript::{update_js_repository, JSConfig};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug)]
pub enum Language {
    Rust,
    JavaScript,
    Python,
}

impl Language {
    /// Try to update dependencies, return dep version so we know when there is a change, or so we have
    /// the deps if the build is bad. Returns a `HashSet` of `pkg - version` strings for O(n)
    /// comparison across versions.
    pub fn try_update(&self, repo: &Repository, local_pth: &Path) -> Result<HashSet<String>> {
        match self {
            Self::JavaScript => update_js_repository(repo, local_pth, None),
            _ => Err(anyhow!(
                "dependency updates are not yet implemented for the language: {:#?}",
                self
            )),
        }
    }

    pub fn try_build(&self, repo: &Repository) -> Result<()> {
        Ok(())
    }
}
