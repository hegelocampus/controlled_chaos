use anyhow::{anyhow, Context, Result};
use git2::{
    Oid,
    build,
    Cred,
    IndexAddOption,
    FetchOptions,
    RemoteCallbacks,
    Repository,
};
use languages::Language;
use std::{
    str,
    env,
    fs::create_dir,
    io::{self, Write},
    path::Path,
    process::Command,
};

fn build_cred_callbacks(ssh_pass: &str) -> RemoteCallbacks {
    // Setup callbacks for ssh
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
            Some(&ssh_pass.to_owned()),
        )
    });
    callbacks
}

fn setup_repo_builder<'a>(ssh_pass: &str) -> build::RepoBuilder {
    let remote_callbacks = build_cred_callbacks(ssh_pass);

    // set fetch options.
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(remote_callbacks);

    // setup builder.
    let mut builder = build::RepoBuilder::new();
    builder.fetch_options(fetch_opts);
    builder
}

// Because the compiler says builder must be passed in as mutable here, I'm concerned the builder
// may not be able to be reused, as I intended. We'll see...
// I need to make sure we pull the most recent changes from the master for already existing
// repositories.
/// Get or create a local checkout of the desired project as a `Repository` struct
fn get_local_checkout(
    mut builder: build::RepoBuilder,
    local_path: &Path,
    project_remote: &str,
) -> Result<Repository> {
    // Best way to do this is probably to maintain a local check out of the repository. First step then
    // is probably to verify that we have that, and if we don't create it.
    let repo = match Repository::open(&local_path) {
        Ok(repo) => Ok(repo),
        // Probably want to verify this error before trying this in a more complete version
        Err(_) => builder.clone(project_remote, &local_path),
    }?;

    Ok(repo)
}


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

fn create_commit(
    repo: &Repository,
) -> Result<Option<Oid>> {
    let mut index = repo.index()?;

    if index.is_empty() {
        // Early return if there are no changed files
        return Ok(None);
    }

    // Update the index to include all file changes.
    index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;

    let oid = index.write_tree()?;
    let tree = repo.find_tree(oid)?;

    let signature = repo.signature()?;
    let parents = repo.find_reference("HEAD")
        .and_then(|x| x.peel_to_commit())?;

    let commit_buff = repo.commit_create_buffer(
        &signature,
        &signature,
        "CCCD: Update dependencies",
        &tree,
        &[&parents],
    )?;
    let commit_str = str::from_utf8(&commit_buff)?;

    let commit_id = repo.commit_signed(
        &commit_str,
        commit_str,
        None,
    )?;
    repo.head()?
        .set_target(commit_id, "CCCD: add signed commit with updated dependencies")?;
    Ok(Some(commit_id))
}

// fn pull_from_remote() -> Result<()> {}

/// find the origin of the git repo, with the following strategy:
/// find the branch that HEAD points to, and read the remote configured for that branch
/// returns the remote and the name of the local branch
fn find_origin(repo: &git2::Repository) -> Result<(git2::Remote, String)> {
    for branch in repo.branches(Some(git2::BranchType::Local))? {
        let b = branch?.0;
        if b.is_head() {
            let parsed_name = &b.name()?.unwrap_or("None");
            let upstream_name_buf = repo.branch_upstream_remote(&format!(
                "refs/heads/{}",
                parsed_name
            ))?;
            let upstream_name = upstream_name_buf
                .as_str().unwrap();
            let origin = repo.find_remote(&upstream_name)?;
            return Ok((origin, parsed_name.to_string()));
        }
    }

    Err(anyhow!("no remotes configured"))
}

// Shamelessly adapted from cortex/ripasso/src/pass.rs
// https://github.com/cortex/ripasso/blob/master/src/pass.rs
fn push_to_remote(
    repo: &Repository,
    commit_id: Oid,
    project_remote: &str,
    origin_branch: &str,
) -> Result<()> {
    println!("commit_id: {:?}", commit_id);
    let mut ref_status = None;
    let (mut origin, branch_name) = find_origin(&repo)?;
    Ok(())

    /*
    let res = {
        let mut opts = git2::PushOptions::new();
        opts.remote_callbacks(callbacks);
        let upstream_name_buf = repo.branch_upstream_remote(hformat!(
                "refs/heads/{}",
        );
        origin.push(&[format!("refs/heads/{}", branch_name)], Some(&mut opts))
    };
    match res {
        Ok(()) if ref_status.is_none() => Ok(()),
        Ok(()) => Err(Error::GenericDyn(format!(
            "failed to push a ref: {:?}",
            ref_status
        ))),
        Err(e) => Err(Error::GenericDyn(format!("failure to push: {}", e))),
    }
*/
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
        Some(id) => push_to_remote(&repo, id, project_remote, "master")?,
        None => println!("No changes were detected"),
    }

    // Deploy

    Ok(())
}
