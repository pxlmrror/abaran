use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn delete(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();
    if let Ok(status) = Command::new("gio")
        .args(["trash", "--", &*path_str])
        .status()
        && status.success()
    {
        return Ok(());
    }
    rm(path)
}

pub fn force_delete(path: &Path) -> Result<()> {
    rm(path)
}

fn rm(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("Failed to delete {}", path.display()))
    } else {
        fs::remove_file(path)
            .with_context(|| format!("Failed to delete {}", path.display()))
    }
}

pub fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)
            .with_context(|| format!("Failed to create {}", dst.display()))?;
        for entry in
            fs::read_dir(src).with_context(|| format!("Failed to read {}", src.display()))?
        {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            copy_recursive(&src_path, &dst_path)?;
        }
        Ok(())
    } else {
        fs::copy(src, dst).with_context(|| {
            format!("Failed to copy {} -> {}", src.display(), dst.display())
        })?;
        Ok(())
    }
}

pub fn rename_entry(src: &Path, dst: &Path) -> Result<()> {
    fs::rename(src, dst).with_context(|| format!("Failed to rename {}", src.display()))
}

pub fn create_file(path: &Path) -> Result<()> {
    fs::File::create(path)
        .with_context(|| format!("Failed to create {}", path.display()))?;
    Ok(())
}

pub fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir(path)
        .with_context(|| format!("Failed to create directory {}", path.display()))?;
    Ok(())
}
