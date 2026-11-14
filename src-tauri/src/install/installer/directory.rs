use super::*;
use anyhow::ensure;
use rayon::prelude::*;
use sha2::{Digest, Sha256};

pub(super) struct DirectoryPlan {
    pub source: PathBuf,
    pub size: u64,
    directories: Vec<PathBuf>,
    files: Vec<DirectoryFile>,
}

struct DirectoryFile {
    path: PathBuf,
    snapshot: FileSnapshot,
}

impl DirectoryPlan {
    pub fn read(source: &Path, control: &TaskControl) -> Result<Self> {
        Self::read_entries(source, &[], control)
    }

    pub fn read_entries(
        source: &Path,
        selected: &[PathBuf],
        control: &TaskControl,
    ) -> Result<Self> {
        let mut plan = Self {
            source: source.to_path_buf(),
            size: 0,
            directories: Vec::new(),
            files: Vec::new(),
        };
        let mut add_entries = |root: &Path| -> Result<()> {
            let entries = walkdir::WalkDir::new(root)
                .follow_links(false)
                .follow_root_links(false)
                .into_iter()
                .filter_entry(|entry| {
                    entry.path().strip_prefix(source).is_ok_and(|path| {
                        !crate::package_artifacts::is_ignored_package_artifact_path(path)
                    })
                });
            for entry in entries {
                check_copy_cancelled(control)?;
                let entry = entry?;
                let path = entry.path().strip_prefix(source)?.to_path_buf();
                if entry.file_type().is_dir() {
                    plan.directories.push(path);
                } else if entry.file_type().is_file() {
                    let snapshot = FileSnapshot::from_metadata(&entry.metadata()?);
                    plan.size = plan.size.saturating_add(snapshot.size);
                    plan.files.push(DirectoryFile { path, snapshot });
                }
            }
            Ok(())
        };
        if selected.is_empty() {
            add_entries(source)?;
        } else {
            for path in selected {
                let path = source.join(path);
                if path.try_exists()? {
                    add_entries(&path)?;
                }
            }
        }
        Ok(plan)
    }

    pub fn contains_file(&self, path: &Path) -> bool {
        self.files.iter().any(|file| file.path == path)
    }

    pub fn copy(
        &self,
        target: &Path,
        control: &TaskControl,
        verify: bool,
        progress: impl Fn(&Path, u64) + Sync,
    ) -> Result<HashMap<String, FileHash>> {
        check_copy_cancelled(control)?;
        ensure!(
            self.source.is_dir(),
            "Source directory is no longer available: {}",
            self.source.display()
        );
        fs::create_dir_all(target)?;
        for directory in &self.directories {
            check_copy_cancelled(control)?;
            fs::create_dir_all(target.join(directory))?;
        }
        let hashes: Vec<Option<FileHash>> = self
            .files
            .par_iter()
            .map_init(
                || vec![0; STREAM_BUFFER_SIZE],
                |buffer, entry| {
                    let input = self.source.join(&entry.path);
                    let output = target.join(&entry.path);
                    let hash = entry.copy(&input, &output, buffer, control, verify)?;
                    progress(&entry.path, entry.snapshot.size);
                    Ok(hash)
                },
            )
            .collect::<Result<_>>()?;
        Ok(hashes
            .into_iter()
            .flatten()
            .map(|hash| (hash.path.clone(), hash))
            .collect())
    }
}

impl DirectoryFile {
    fn copy(
        &self,
        source: &Path,
        target: &Path,
        buffer: &mut [u8],
        control: &TaskControl,
        verify: bool,
    ) -> Result<Option<FileHash>> {
        let check_source = || -> Result<()> {
            let meta = fs::symlink_metadata(source).with_context(|| {
                format!(
                    "Source file changed during installation: {}",
                    source.display()
                )
            })?;
            ensure!(
                meta.is_file() && FileSnapshot::from_metadata(&meta) == self.snapshot,
                "Source file changed during installation: {}",
                source.display()
            );
            Ok(())
        };
        check_copy_cancelled(control)?;
        check_source()?;
        let mut input = open_file_for_read_with_retry(source)
            .with_context(|| format!("Failed to open source file: {}", source.display()))?;
        ensure!(
            FileSnapshot::from_metadata(&input.metadata()?) == self.snapshot,
            "Source file changed during installation: {}",
            source.display()
        );
        let mut output = fs::File::create(target)
            .with_context(|| format!("Failed to create target file: {}", target.display()))?;
        let mut hasher = verify.then(Sha256::new);
        let copied = copy_with_buffer(&mut input, &mut output, buffer, |bytes| {
            check_copy_cancelled(control)?;
            if let Some(hasher) = &mut hasher {
                hasher.update(bytes);
            }
            Ok(())
        })?;
        ensure!(
            copied == self.snapshot.size
                && FileSnapshot::from_metadata(&input.metadata()?) == self.snapshot,
            "Source file changed during installation: {}",
            source.display()
        );
        check_source()?;
        let _ = remove_readonly_attribute(target);
        Ok(hasher.map(|hasher| FileHash {
            path: self.path.to_string_lossy().replace('\\', "/"),
            hash: format!("{:x}", hasher.finalize()),
            algorithm: HashAlgorithm::Sha256,
        }))
    }
}

pub(super) fn check_copy_cancelled(control: &TaskControl) -> std::io::Result<()> {
    if control.is_cancelled() {
        return Err(std::io::Error::other("Cancelled by user"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_copy_hashes_only_installed_files_and_verifies_the_destination() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        fs::write(source.path().join("plane.acf"), b"abc").unwrap();
        fs::write(source.path().join("Thumbs.db"), b"metadata").unwrap();
        let control = TaskControl::new();
        let plan = DirectoryPlan::read(source.path(), &control).unwrap();
        fs::write(source.path().join("added-later.txt"), b"new").unwrap();
        let hashes = plan.copy(target.path(), &control, true, |_, _| {}).unwrap();
        assert_eq!(plan.size, 3);
        assert_eq!(hashes.len(), 1);
        assert_eq!(
            hashes["plane.acf"].hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(!target.path().join("added-later.txt").exists());
        assert!(!target.path().join("Thumbs.db").exists());
        let verifier = crate::verifier::FileVerifier::new();
        assert!(verifier
            .verify_files(target.path(), &hashes)
            .unwrap()
            .is_empty());
        fs::write(target.path().join("plane.acf"), b"bad").unwrap();
        assert_eq!(
            verifier.verify_files(target.path(), &hashes).unwrap().len(),
            1
        );
    }

    #[test]
    fn planned_copy_rejects_a_changed_source_and_respects_cancel() {
        let source = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let file = source.path().join("plane.acf");
        fs::write(&file, b"abc").unwrap();
        let control = TaskControl::new();
        let plan = DirectoryPlan::read(source.path(), &control).unwrap();
        fs::write(&file, b"changed length").unwrap();
        let error = plan
            .copy(target.path(), &control, false, |_, _| {})
            .unwrap_err();
        assert!(error.to_string().contains("Source file changed"));
        assert!(!target.path().join("plane.acf").exists());

        control.request_cancel_all();
        let error = plan
            .copy(target.path(), &control, true, |_, _| {})
            .unwrap_err();
        assert!(error.to_string().contains("Cancelled by user"));
        assert!(DirectoryPlan::read(source.path(), &control).is_err());
    }
}
