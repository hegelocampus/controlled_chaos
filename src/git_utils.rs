use git2::{build, Cred, FetchOptions, IndexAddOption, Oid, RemoteCallbacks, Repository};
use anyhow::{anyhow, Result};
use std::{
    env,
    str,
    path::Path,
};

// fn pull_from_remote() -> Result<()> {}

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

pub fn setup_repo_builder<'a>(ssh_pass: &str) -> build::RepoBuilder {
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
pub fn get_local_checkout(
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

/// Returns a gpg signature for the supplied string. Suitable to add to a gpg commit.
fn gpg_sign_string(commit: &str) -> Result<String> {
    let config = git2::Config::open_default()?;

    let signing_key = config.get_string("user.signingkey")?;

    let mut ctx = gpgme::Context::from_protocol(gpgme::Protocol::OpenPgp)?;
    ctx.set_armor(true);
    let key = ctx.get_secret_key(signing_key)?;

    ctx.add_signer(&key)?;
    let mut output = Vec::new();
    ctx.sign_detached(commit, &mut output)?;

    Ok(String::from_utf8(output)?)
}

pub fn create_commit(repo: &Repository) -> Result<Option<Oid>> {
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
    let parents = repo
        .find_reference("HEAD")
        .and_then(|x| x.peel_to_commit())?;

    let commit_buff = repo.commit_create_buffer(
        &signature,
        &signature,
        "CCCD: Update dependencies",
        &tree,
        &[&parents],
    )?;
    let commit_str = str::from_utf8(&commit_buff)?;
    let gpg_sig = gpg_sign_string(commit_str)?;

    let commit_id = repo.commit_signed(&commit_str, &gpg_sig, Some("gpgsig"))?;
    repo.head()?.set_target(
        commit_id,
        "CCCD: add signed commit with updated dependencies",
    )?;
    Ok(Some(commit_id))
}

/// find the origin of the git repo, with the following strategy:
/// find the branch that HEAD points to, and read the remote configured for that branch
/// returns the remote and the name of the local branch
fn find_origin(repo: &git2::Repository) -> Result<(git2::Remote, String)> {
    for branch in repo.branches(Some(git2::BranchType::Local))? {
        let b = branch?.0;
        if b.is_head() {
            let parsed_name = &b.name()?.unwrap_or("None");
            let upstream_name_buf =
                repo.branch_upstream_remote(&format!("refs/heads/{}", parsed_name))?;
            let upstream_name = upstream_name_buf.as_str().unwrap();
            let origin = repo.find_remote(&upstream_name)?;
            return Ok((origin, parsed_name.to_string()));
        }
    }

    Err(anyhow!("no remotes configured"))
}

// Shamelessly adapted/stolen from cortex/ripasso/src/pass.rs
// https://github.com/cortex/ripasso/blob/master/src/pass.rs
pub fn push_to_remote(
    repo: &Repository,
    commit_id: Oid,
    ssh_pass: &str,
) -> Result<()> {
    println!("commit_id: {:?}", commit_id);
    let (mut origin, branch_name) = find_origin(&repo)?;
    let mut ref_status = None;
    let res = {
        let mut remote_callbacks = build_cred_callbacks(ssh_pass);
        remote_callbacks.push_update_reference(|refname, status| {
            assert_eq!(refname, format!("refs/heads/{}", branch_name));
            ref_status = status.map(|s| s.to_string());
            Ok(())
        });

        let mut opts = git2::PushOptions::new();
        opts.remote_callbacks(remote_callbacks);
        origin.push(&[format!("refs/heads/{}", branch_name)], Some(&mut opts))
    };
    match res {
        Ok(()) if ref_status.is_none() => Ok(()),
        Ok(()) => Err(anyhow!(
            "failed to push a ref: {:?}",
            ref_status
        )),
        Err(e) => Err(anyhow!(format!("failure to push: {}", e))),
    }
}

