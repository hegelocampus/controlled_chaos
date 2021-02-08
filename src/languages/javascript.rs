use std::str;
use std::io::{self, Write, stdout, stderr};
use regex::Regex;
use std::path::Path;
use std::process::Command;
use std::collections::HashSet;
use json::parse;
use anyhow::{Context, anyhow, Result};
use git2::Repository;

// This will probably contain things like if the project uses npm or yarn and packages to explictly
// **not** update. May be a good idea to inherit from some meta config struct in the future.
// Probably want to use the Default module for this to set reasonable default values.
pub struct JSConfig {

}


pub fn update_js_repository(repo: &Repository, local_pth: &Path, _cfg: Option<JSConfig>) -> Result<HashSet<String>> {
    let output = Command::new("yarn")
        .current_dir(local_pth)
        .arg("upgrade")
        .arg("--json")
        .arg("--non-interactive")
        .output()?;

    let yarn_depends: Regex = Regex::new(r#"\{"type":"tree","data":\{"type":"newAllDependencies""#).unwrap();
    let depends_str = str::from_utf8(&output.stdout)?
        .split_whitespace()
        .find(|line| yarn_depends.is_match(&line))
        .context("could not find dependency JSON object \"newAllDependencies\"")?;
    // This is the first dependency object
    let parsed: HashSet<String> = parse(depends_str)?["data"]["trees"].members_mut()
        .filter_map(|dep| dep["name"].take_string())
        .collect();
    println!("{:#?}", parsed);
    //stdout().write_all(&output.stdout)?;
    //stderr().write_all(&output.stderr)?;
    Ok(parsed)
}
