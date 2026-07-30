use std::path::{Path, PathBuf};

use crate::error::Result;

pub fn get_cache_dir(base: &str) -> PathBuf {
    Path::new(base).to_path_buf()
}

pub fn ensure_cache_dir(base: &str) -> Result<PathBuf> {
    let dir = get_cache_dir(base);
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}
