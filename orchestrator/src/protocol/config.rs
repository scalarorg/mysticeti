use std::fs;
use std::path::{Path, PathBuf};

use consensus_config::AuthorityIndex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PrivateConfig {
    authority_index: AuthorityIndex,
    storage_path: StorageDir,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct StorageDir {
    path: PathBuf,
}

impl PrivateConfig {
    pub fn new(path: PathBuf, authority_index: AuthorityIndex) -> Self {
        fs::create_dir_all(&path).expect("Failed to create validator storage directory");
        Self {
            authority_index,
            storage_path: StorageDir { path },
        }
    }
    pub fn new_for_benchmarks(dir: &Path, authority_index: AuthorityIndex) -> Self {
        // TODO: Once we have a crypto library, generate a keypair from a fixed seed.
        tracing::warn!("Generating a predictable keypair for benchmarking");
        let path = dir.join(format!("val-{authority_index}"));
        fs::create_dir_all(&path).expect("Failed to create validator storage directory");
        Self {
            authority_index,
            storage_path: StorageDir { path },
        }
    }

    pub fn default_filename(authority: AuthorityIndex) -> PathBuf {
        ["private", &format!("{authority}.yaml")].iter().collect()
    }

    pub fn storage(&self) -> &StorageDir {
        &self.storage_path
    }
}
