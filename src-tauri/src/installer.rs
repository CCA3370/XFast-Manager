use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::InstallTask;

pub struct Installer;

impl Installer {
    pub fn new() -> Self {
        Installer
    }

    /// Install a list of tasks
    pub fn install(&self, tasks: Vec<InstallTask>) -> Result<()> {
        for task in tasks {
            self.install_task(&task)?;
        }
        Ok(())
    }

    /// Install a single task
    fn install_task(&self, task: &InstallTask) -> Result<()> {
        let source = Path::new(&task.source_path);
        let target = Path::new(&task.target_path);

        // Create parent directory if it doesn't exist
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create target directory: {:?}", parent))?;
        }

        // Check if source is a directory or archive
        if source.is_dir() {
            self.copy_directory(source, target)?;
        } else if source.is_file() {
            // Extract archive
            self.extract_archive(source, target)?;
        } else {
            return Err(anyhow::anyhow!("Source path is neither file nor directory"));
        }

        Ok(())
    }

    /// Copy a directory recursively
    fn copy_directory(&self, source: &Path, target: &Path) -> Result<()> {
        if !target.exists() {
            fs::create_dir_all(target)?;
        }

        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let source_path = entry.path();
            let file_name = entry.file_name();
            let target_path = target.join(&file_name);

            if file_type.is_dir() {
                self.copy_directory(&source_path, &target_path)?;
            } else {
                fs::copy(&source_path, &target_path)
                    .context(format!("Failed to copy {:?} to {:?}", source_path, target_path))?;
            }
        }

        Ok(())
    }

    /// Extract an archive to target directory
    fn extract_archive(&self, archive: &Path, target: &Path) -> Result<()> {
        let extension = archive
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension"))?;

        match extension {
            "zip" => self.extract_zip(archive, target)?,
            "7z" => self.extract_7z(archive, target)?,
            _ => return Err(anyhow::anyhow!("Unsupported archive format: {}", extension)),
        }

        Ok(())
    }

    /// Extract ZIP archive
    fn extract_zip(&self, archive: &Path, target: &Path) -> Result<()> {
        use zip::ZipArchive;
        
        let file = fs::File::open(archive)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => target.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }

            // Set permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }

        Ok(())
    }

    /// Extract 7z archive
    fn extract_7z(&self, archive: &Path, target: &Path) -> Result<()> {
        use sevenz_rust::decompress_file;
        
        decompress_file(archive, target)
            .map_err(|e| anyhow::anyhow!("Failed to extract 7z: {}", e))?;

        Ok(())
    }
}
