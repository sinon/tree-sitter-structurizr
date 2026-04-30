use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

/// Creates project-local temporary workspaces under `tmp/` so failing runs are
/// easy to inspect and stale test directories can be cleaned up deterministically.
pub struct RepoLocalTempWorkspace {
    root: PathBuf,
}

impl RepoLocalTempWorkspace {
    pub fn new(prefix: &str, name: &str) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repository root should canonicalize");
        let tmp_root = repo_root.join("tmp");
        let stale_prefix = format!("{prefix}-{name}-");
        if let Ok(entries) = fs::read_dir(&tmp_root) {
            for entry in entries.flatten() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(&stale_prefix)
                {
                    let _ = fs::remove_dir_all(entry.path());
                }
            }
        }

        let root = tmp_root.join(format!("{prefix}-{name}-{}-{id}", std::process::id()));

        if root.exists() {
            fs::remove_dir_all(&root).unwrap_or_else(|error| {
                panic!(
                    "failed to clear project-local temp workspace `{}`: {error}",
                    root.display()
                )
            });
        }
        fs::create_dir_all(&root).unwrap_or_else(|error| {
            panic!(
                "failed to create project-local temp workspace `{}`: {error}",
                root.display()
            )
        });

        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn file_path(&self, relative_path: &str) -> PathBuf {
        self.root.join(relative_path)
    }

    pub fn write_file(&self, relative_path: &str, source: &str) {
        self.write_bytes(relative_path, source.as_bytes());
    }

    pub fn write_bytes(&self, relative_path: &str, bytes: &[u8]) {
        let path = self.file_path(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|error| {
                panic!(
                    "failed to create project-local temp parent `{}`: {error}",
                    parent.display()
                )
            });
        }
        fs::write(&path, bytes).unwrap_or_else(|error| {
            panic!(
                "failed to write project-local temp file `{}`: {error}",
                path.display()
            )
        });
    }
}

impl Drop for RepoLocalTempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
