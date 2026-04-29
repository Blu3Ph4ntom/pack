//! Cache layout and operations.

use directories::ProjectDirs;
use pack_core::{PackError, PackResult};
use std::path::PathBuf;

pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new() -> PackResult<Self> {
        let proj_dirs = ProjectDirs::from("com", "piper", "pack")
            .ok_or_else(|| PackError::Cache("could not determine cache directory".into()))?;

        let root = proj_dirs.cache_dir().to_path_buf();
        Ok(Self { root })
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn packages_dir(&self) -> PathBuf {
        self.root.join("packages")
    }

    pub fn metadata_dir(&self) -> PathBuf {
        self.root.join("metadata")
    }

    pub fn native_dir(&self) -> PathBuf {
        self.root.join("native")
    }

    pub fn ensure_dirs(&self) -> PackResult<()> {
        std::fs::create_dir_all(self.packages_dir())?;
        std::fs::create_dir_all(self.metadata_dir())?;
        std::fs::create_dir_all(self.native_dir())?;
        Ok(())
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new().expect("failed to create default cache")
    }
}
