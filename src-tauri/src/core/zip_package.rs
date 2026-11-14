use std::fs::{File, Metadata};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::archive_input::{prepare_archive_for_read, ArchiveFormat, PreparedArchive};
use crate::archive_reader::ArchiveFiles;
pub(crate) use crate::archive_reader::ArchiveReader as ZipReader;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSnapshot {
    pub size: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) volumes: Vec<FileSnapshot>,
    modified: Option<SystemTime>,
    created: Option<SystemTime>,
    #[cfg(unix)]
    inode: String,
    #[cfg(unix)]
    device: String,
    #[cfg(windows)]
    file_id: Option<String>,
}

impl FileSnapshot {
    pub fn from_metadata(metadata: &Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        Self {
            size: metadata.len(),
            volumes: Vec::new(),
            modified: metadata.modified().ok(),
            created: metadata.created().ok(),
            #[cfg(unix)]
            inode: metadata.ino().to_string(),
            #[cfg(unix)]
            device: metadata.dev().to_string(),
            #[cfg(windows)]
            file_id: None,
        }
    }

    pub(crate) fn from_file(file: &File) -> io::Result<Self> {
        let snapshot = Self::from_metadata(&file.metadata()?);
        #[cfg(windows)]
        {
            use std::mem::MaybeUninit;
            use std::os::windows::io::AsRawHandle;
            use winapi::um::fileapi::GetFileInformationByHandle;

            let mut info = MaybeUninit::uninit();
            // The handle stays open and the API initializes the output on success.
            let file_id = unsafe {
                if GetFileInformationByHandle(file.as_raw_handle().cast(), info.as_mut_ptr()) != 0 {
                    let info = info.assume_init();
                    let index =
                        (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow);
                    Some(format!("{:08x}:{index:016x}", info.dwVolumeSerialNumber))
                } else {
                    None
                }
            };
            Ok(Self {
                file_id,
                ..snapshot
            })
        }
        #[cfg(not(windows))]
        Ok(snapshot)
    }
}

pub(crate) struct ZipEntry {
    pub index: usize,
    pub name: String,
    pub enclosed_path: Option<PathBuf>,
    pub size: u64,
    pub is_directory: bool,
    pub encrypted: bool,
    pub crc32: u32,
    #[cfg(unix)]
    pub unix_mode: Option<u32>,
}

pub(crate) struct ZipSummary {
    pub source: Option<FileSnapshot>,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

pub(crate) struct ZipPackage {
    pub source_path: PathBuf,
    archive: ZipArchive<ZipReader>,
    pub entries: Vec<ZipEntry>,
    input: ZipReader,
    source: Option<(ArchiveFiles, PreparedArchive)>,
}

impl ZipPackage {
    pub fn open(path: &Path, expected: Option<&FileSnapshot>) -> Result<Self> {
        let prepared = prepare_archive_for_read(path, ArchiveFormat::Zip)?;
        let files = prepared
            .files()
            .with_context(|| Self::changed_message(path))?;
        let snapshot = files.revision();
        ensure!(
            expected.is_none_or(|expected| !prepared.is_temporary() && expected == &snapshot),
            Self::changed_message(path)
        );
        let mut package = Self::from_reader(path, files.reader())?;
        package.source = Some((files, prepared));
        Ok(package)
    }

    pub fn from_reader(path: &Path, input: ZipReader) -> Result<Self> {
        let mut archive = ZipArchive::new(input.clone())?;
        let mut entries = Vec::with_capacity(archive.len());
        for index in 0..archive.len() {
            let file = archive
                .by_index_raw(index)
                .with_context(|| format!("Failed to read ZIP entry {index}"))?;
            entries.push(ZipEntry {
                index,
                name: file.name().replace('\\', "/"),
                enclosed_path: file.enclosed_name(),
                size: file.size(),
                is_directory: file.is_dir(),
                encrypted: file.encrypted(),
                crc32: file.crc32(),
                #[cfg(unix)]
                unix_mode: file.unix_mode(),
            });
        }
        Ok(Self {
            source_path: path.to_path_buf(),
            archive,
            entries,
            input,
            source: None,
        })
    }

    pub fn reader(&self) -> ZipArchive<ZipReader> {
        self.archive.clone()
    }

    pub fn input(&self) -> ZipReader {
        self.input.clone()
    }

    pub fn summary(&self) -> ZipSummary {
        ZipSummary {
            source: self
                .source
                .as_ref()
                .filter(|(_, prepared)| !prepared.is_temporary())
                .map(|(files, _)| files.revision()),
            compressed_size: self.input.len(),
            uncompressed_size: self
                .entries
                .iter()
                .fold(0u64, |size, entry| size.saturating_add(entry.size)),
        }
    }

    pub fn check_revision(&self, expected: Option<&FileSnapshot>) -> Result<()> {
        ensure!(
            expected.is_none_or(|expected| self.source.as_ref().is_some_and(
                |(files, prepared)| { !prepared.is_temporary() && expected == &files.revision() }
            )),
            Self::changed_message(&self.source_path)
        );
        Ok(())
    }

    pub fn check_source(&self) -> Result<()> {
        self.input
            .check_source()
            .with_context(|| Self::changed_message(&self.source_path))?;
        Ok(())
    }

    fn changed_message(path: &Path) -> String {
        format!(
            "ZIP package changed. Analyze it again before installing: {}",
            path.display()
        )
    }
}

pub(crate) fn stored_member(
    archive: &mut ZipArchive<ZipReader>,
    input: &ZipReader,
    index: usize,
) -> Result<Option<ZipReader>> {
    let entry = archive.by_index_raw(index)?;
    if entry.encrypted() || entry.compression() != zip::CompressionMethod::Stored {
        return Ok(None);
    }
    ensure!(
        entry.compressed_size() == entry.size(),
        "Invalid stored ZIP member size"
    );
    Ok(Some(input.range(entry.data_start(), entry.size())?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};
    use zip::write::SimpleFileOptions;

    pub(crate) fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, content) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(content).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn reused_zip_index_reads_independent_entries_concurrently() {
        use rayon::prelude::*;

        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("two-addons.zip");
        let names: Vec<_> = (0..400).map(|index| format!("Addon/{index}.txt")).collect();
        let entries: Vec<_> = names
            .iter()
            .map(|name| (name.as_str(), name.as_bytes()))
            .collect();
        write_zip(&path, &entries);
        let zip = ZipPackage::open(&path, None).unwrap();

        names.par_iter().enumerate().for_each_init(
            || zip.reader(),
            |reader, (index, name)| {
                let mut content = String::new();
                reader
                    .by_index(index)
                    .unwrap()
                    .read_to_string(&mut content)
                    .unwrap();
                assert_eq!(&content, name);
            },
        );
    }

    #[test]
    fn replacing_an_analyzed_zip_requires_reanalysis() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("addon.zip");
        write_zip(&path, &[("Addon/plane.acf", b"old")]);
        let zip = ZipPackage::open(&path, None).unwrap();
        let revision = zip.summary().source.unwrap();
        assert!(ZipPackage::open(&path, Some(&revision)).is_ok());
        fs::rename(&path, temp.path().join("original.zip")).unwrap();
        write_zip(&path, &[("Addon/plane.acf", b"replacement")]);

        assert!(zip.check_source().is_err());
        let error = ZipPackage::open(&path, Some(&revision)).err().unwrap();
        assert!(error.to_string().contains("Analyze it again"));
    }

    #[test]
    fn malformed_zip_fails_indexing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.zip");
        fs::write(&path, b"PK\x03\x04").unwrap();
        assert!(ZipPackage::open(&path, None).is_err());
    }
}

#[cfg(test)]
pub(crate) use tests::write_zip;
