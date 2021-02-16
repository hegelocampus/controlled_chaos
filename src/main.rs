pub(crate) mod git_utils;

use crate::git_utils::{create_commit, get_local_checkout, push_to_remote, setup_repo_builder};
use anyhow::{anyhow, Context, Result};
use languages::Language;
use std::{
    env,
    fs::create_dir,
    io::{self, Write},
    path::Path,
    process::Command,
    str,
};

fn run_tests(local_path: &Path, tests: Vec<&str>) -> Result<()> {
    for test in tests.iter() {
        let cmd_parts: Vec<&str> = test.split(' ').collect();
        let output = Command::new(cmd_parts[0])
            .current_dir(local_path)
            .args(&cmd_parts[1..])
            .output()?;
        if !output.status.success() {
            eprintln!("Error: test \"{}\" failed with the following output:", test);
            io::stderr().write_all(&output.stderr).unwrap();
            return Err(anyhow!("The following test Test failed: {}", test));
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let should_test = false;
    let base_path = Path::new("./.local_checkouts/");
    if !base_path.exists() {
        // If this fails it will be because the parent doesn't exist, which would mean someting is
        // seriously wrong, or the current user doesn't have permission to create the directory.
        create_dir(base_path)
            .context(format!("could not create directory at {:#?}, are you sure you have correct permissions to read and write that file?", base_path))?;
    }

    // This will all go in a JSON/YAML file
    let project_name = "portfolio";
    let project_remote = "git@github.com:hegelocampus/portfolio.git";
    let test_steps = vec!["yarn test"];
    let deployment_steps = vec!["yarn gulp build", "firebase deploy"];
    let project_language = Language::JavaScript;
    let local_path = base_path.join(project_name);

    let ssh_pass = &env::var("CCCI_SSH_PASS")
        .context("CCCI_SSH_PASS environment variable is not defined, please define this to use ssh git remote URLs")?;

    // This builder may be reused for all repositories
    println!(
        "Finding or fetching local repository for {}...",
        project_name
    );
    let builder = setup_repo_builder(&ssh_pass);
    let repo = get_local_checkout(builder, &local_path, project_remote)?;

    println!("Atempting to update {}...", project_name);

    let _new_dep_versions = project_language.try_update(&repo, &local_path)?;
    // TODO: Check new_dep_versions against known bad versions

    //Test
    if should_test {
        println!("Update succeeded! Running tests...");
        run_tests(&local_path, test_steps)?;
    } else {
        println!("Update succeeded! Skipping tests");
    }

    println!("Tests succeeded! Commiting changes to remote...");

    // Commit changes
    let commit_id = create_commit(&repo)?;
    match commit_id {
        Some(id) => push_to_remote(&repo, id, &ssh_pass)?,
        None => println!("No changes were detected"),
    }

    println!("Changes have been pushed to remote! Deploying changes...");

    // Deploy

    Ok(())
}
