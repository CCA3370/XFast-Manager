use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::zip_package::FileSnapshot;

#[derive(Clone)]
pub(crate) struct ArchiveFiles {
    parts: Arc<[ArchivePart]>,
    len: u64,
}

struct ArchivePart {
    path: PathBuf,
    end: u64,
    revision: FileSnapshot,
}

impl ArchiveFiles {
    pub fn open(paths: impl IntoIterator<Item = PathBuf>) -> io::Result<Self> {
        let mut end = 0u64;
        let mut parts = Vec::new();
        for path in paths {
            let revision = FileSnapshot::from_file(&File::open(&path)?)?;
            end = end.checked_add(revision.size).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Archive size overflow")
            })?;
            parts.push(ArchivePart {
                path,
                end,
                revision,
            });
        }
        if parts.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Archive has no source files",
            ));
        }
        Ok(Self {
            parts: parts.into(),
            len: end,
        })
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn revision(&self) -> FileSnapshot {
        let mut revision = self.parts[0].revision.clone();
        if self.parts.len() > 1 {
            revision.volumes = self.parts[1..]
                .iter()
                .map(|part| part.revision.clone())
                .collect();
        }
        revision
    }

    pub fn check_source(&self) -> io::Result<()> {
        for part in self.parts.iter() {
            part.open()?;
        }
        Ok(())
    }

    pub fn reader(&self) -> ArchiveReader {
        ArchiveReader {
            files: self.clone(),
            open_part: None,
            start: 0,
            len: self.len,
            position: 0,
        }
    }
}

impl ArchivePart {
    fn open(&self) -> io::Result<File> {
        let file = File::open(&self.path)?;
        if FileSnapshot::from_file(&file)? != self.revision {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Archive source changed: {}", self.path.display()),
            ));
        }
        Ok(file)
    }
}

pub(crate) struct ArchiveReader {
    files: ArchiveFiles,
    open_part: Option<(usize, File)>,
    start: u64,
    len: u64,
    position: u64,
}

impl Clone for ArchiveReader {
    fn clone(&self) -> Self {
        Self {
            files: self.files.clone(),
            open_part: None,
            start: self.start,
            len: self.len,
            position: self.position,
        }
    }
}

impl ArchiveReader {
    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn check_source(&self) -> io::Result<()> {
        self.files.check_source()
    }

    pub fn revision(&self) -> FileSnapshot {
        self.files.revision()
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(ArchiveFiles::open([path.to_path_buf()])?.reader())
    }

    pub fn range(&self, offset: u64, len: u64) -> io::Result<Self> {
        if offset > self.len || len > self.len - offset {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Archive member extends beyond its source",
            ));
        }
        Ok(Self {
            files: self.files.clone(),
            open_part: None,
            start: self.start + offset,
            len,
            position: 0,
        })
    }
}

impl Read for ArchiveReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() || self.position >= self.len {
            return Ok(0);
        }
        let offset = self.start + self.position;
        let parts = &self.files.parts;
        let index = parts.partition_point(|part| part.end <= offset);
        let part = &parts[index];
        let part_start = index
            .checked_sub(1)
            .map_or(0, |previous| parts[previous].end);
        if self
            .open_part
            .as_ref()
            .is_none_or(|(open, _)| *open != index)
        {
            self.open_part = Some((index, part.open()?));
        }
        let (_, file) = self.open_part.as_mut().expect("archive part is open");
        let limit = (buffer.len() as u64)
            .min(part.end - offset)
            .min(self.len - self.position);
        let input = &mut buffer[..limit as usize];
        #[cfg(unix)]
        let count = std::os::unix::fs::FileExt::read_at(file, input, offset - part_start)?;
        #[cfg(windows)]
        let count = std::os::windows::fs::FileExt::seek_read(file, input, offset - part_start)?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Archive part was truncated",
            ));
        }
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for ArchiveReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.position = match position {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(offset) => self.len.checked_add_signed(offset),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid archive seek"))?;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_and_independent_cursors_cross_volume_boundaries() -> io::Result<()> {
        let dir = tempfile::tempdir()?;
        let first = dir.path().join("test.001");
        let second = dir.path().join("test.002");
        std::fs::write(&first, b"abcd")?;
        std::fs::write(&second, b"efgh")?;
        let files = ArchiveFiles::open([first, second.clone()])?;
        let mut reader = files.reader();
        let mut range = reader.range(2, 4)?;
        let mut bytes = Vec::new();
        range.read_to_end(&mut bytes)?;
        assert_eq!(bytes, b"cdef");
        assert_eq!(reader.stream_position()?, 0);
        reader.seek(SeekFrom::End(-2))?;
        let mut clone = reader.clone();
        assert_eq!(reader.seek(SeekFrom::Current(-6))?, 0);
        bytes.clear();
        clone.read_to_end(&mut bytes)?;
        assert_eq!(bytes, b"gh");
        assert!(reader.range(7, 2).is_err());
        std::fs::write(second, b"replacement")?;
        assert!(files.check_source().is_err());
        Ok(())
    }
}
