use std::collections::HashSet;
use anyhow::{Result};
use git2::Repository;

// This will probably contain things like if the project uses npm or yarn and packages to explictly
// **not** update. May be a good idea to inherit from some meta config struct in the future.
// Probably want to use the Default module for this to set reasonable default values.
pub struct JSConfig {


}

pub fn update_js_repository(repo: &Repository, cfg: Option<JSConfig>) -> Result<HashSet<String>> {
    let res = HashSet::new();
    Ok(res)
}
