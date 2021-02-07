use languages::javascript::{update_js_repository};
use git2::{Repository};

#[derive(Debug)]
enum Language {
    Rust,
    JavaScript,
    Python,
    Ruby,
}

impl Language {
    /// Try to update dependencies, return dep version so we know when there is a change, or so we have
    /// the deps if the build is bad. Returns a `HashSet` of `pkg - version` strings for O(n)
    /// comparison across versions.
    pub fn try_update(&self, repo: Repository) -> Result<HashSet<String>> {
        match self {
            Self::JavaScript => update_js_repository(repo),
            _ => Err(anyhow!("dependency updates are not yet implemented for the language: {}", self)),
        }
    }

    pub fn try_build(&self, repo: Repository) -> Result<()> {
        Ok()
    }
}

