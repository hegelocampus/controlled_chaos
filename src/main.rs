use anyhow::{anyhow, Context, Result};
use git2::{build, Cred, FetchOptions, RemoteCallbacks, Repository};
use languages::Language;
use std::env;
use std::fs::create_dir;
use std::path::Path;

fn setup_repo_builder(ssh_pass: &str) -> build::RepoBuilder {
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

    // set fetch options.
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);

    // setup builder.
    let mut builder = build::RepoBuilder::new();
    builder.fetch_options(fetch_opts);
    builder
}

// Because the compiler says builder must be passed in as mutable here, I'm concerned the builder
// may not be able to be reused, as I intended. We'll see...
/// Get or create a local checkout of the desired project as a `Repository` struct
fn get_local_checkout(
    mut builder: build::RepoBuilder,
    local_path: &Path,
    project_name: &str,
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

fn main() -> Result<()> {
    let base_path = Path::new("./.local_checkouts/");
    if !base_path.exists() {
        // If this fails it will be because the parent doesn't exist, which would mean someting is
        // seriously wrong, or the current user doesn't have permission to create the directory.
        create_dir(base_path)?;
    }

    // This will all go in a JSON/YAML file
    let project_name = "portfolio";
    let project_remote = "git@github.com:hegelocampus/portfolio.git";
    let project_language = Language::JavaScript;
    let local_path = base_path.join(project_name);

    let ssh_pass = &env::var("CCCI_SSH_PASS").context("CCCI_SSH_PASS environment variable is not defined, please define this to use ssh git remote URLs")?;
    // This builder will be reused for all repositories
    let builder = setup_repo_builder(&ssh_pass);
    let repo = get_local_checkout(builder, &local_path, project_name, project_remote)?;

    let new_dep_versions = project_language.try_update(&repo, &local_path)?;

    Ok(())
}
