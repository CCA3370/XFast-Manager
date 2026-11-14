use std::collections::{BTreeMap, HashMap};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{ensure, Context, Result};
use sevenz_rust2::{Archive, ArchiveEntry, BlockDecoder, Password};

use crate::archive_input::{prepare_archive_for_read, ArchiveFormat, PreparedArchive};
use crate::archive_reader::ArchiveReader;

pub(crate) struct SevenzPackage {
    pub archive: Archive,
    pub source_path: Option<PathBuf>,
    pub prepared: Option<PreparedArchive>,
    input: ArchiveReader,
    password: Password,
    names: HashMap<String, usize>,
}

impl SevenzPackage {
    pub fn open(path: &Path, password: Option<&str>) -> Result<Self> {
        let prepared = prepare_archive_for_read(path, ArchiveFormat::SevenZ)?;
        let files = prepared.files()?;
        let mut package = Self::from_reader(files.reader(), password)?;
        package.source_path = Some(path.to_path_buf());
        package.prepared = Some(prepared);
        Ok(package)
    }

    pub fn from_reader(input: ArchiveReader, password: Option<&str>) -> Result<Self> {
        let password = password.map(Password::from).unwrap_or_else(Password::empty);
        let archive = Archive::read(
            &mut BufReader::with_capacity(256 * 1024, input.clone()),
            &password,
        )?;
        let names = archive
            .files
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.name.replace('\\', "/"), index))
            .collect();
        Ok(Self {
            archive,
            input,
            password,
            names,
            source_path: None,
            prepared: None,
        })
    }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.names.get(name).copied()
    }

    pub fn check_source(&self) -> Result<()> {
        Ok(self.input.check_source()?)
    }

    pub fn revision(&self) -> crate::zip_package::FileSnapshot {
        self.input.revision()
    }

    pub fn compressed_size(&self) -> u64 {
        self.input.len()
    }

    pub fn stored_member(&self, index: usize) -> Result<Option<ArchiveReader>> {
        let entry = &self.archive.files[index];
        let Some(block_index) = self.archive.stream_map.file_block_index[index] else {
            return Ok(None);
        };
        let block = &self.archive.blocks[block_index];
        if block.coders.len() != 1
            || block.coders[0].encoder_method_id() != sevenz_rust2::EncoderMethod::ID_COPY
        {
            return Ok(None);
        }
        let map = &self.archive.stream_map;
        let stream = map.block_first_pack_stream_index()[block_index];
        let first_file = map.block_first_file_index[block_index];
        let preceding = self.archive.files[first_file..index]
            .iter()
            .try_fold(0u64, |offset, entry| offset.checked_add(entry.size))
            .context("Invalid stored 7z member offset")?;
        let stream_size = self.archive.pack_sizes()[stream];
        ensure!(
            preceding <= stream_size && entry.size <= stream_size - preceding,
            "Invalid stored 7z member size"
        );
        let offset = sevenz_rust2::SIGNATURE_HEADER_SIZE
            .checked_add(self.archive.pack_pos())
            .and_then(|offset| offset.checked_add(map.pack_stream_offsets()[stream]))
            .and_then(|offset| offset.checked_add(preceding))
            .context("Invalid stored 7z member range")?;
        Ok(Some(self.input.range(offset, entry.size)?))
    }

    pub fn groups(
        &self,
        indices: impl IntoIterator<Item = usize>,
    ) -> BTreeMap<Option<usize>, Vec<usize>> {
        let mut groups = BTreeMap::<_, Vec<_>>::new();
        for index in indices {
            groups
                .entry(
                    self.archive.files[index]
                        .has_stream
                        .then_some(self.archive.stream_map.file_block_index[index])
                        .flatten(),
                )
                .or_default()
                .push(index);
        }
        for indices in groups.values_mut() {
            indices.sort_unstable();
            indices.dedup();
        }
        groups
    }

    pub fn read_group(
        &self,
        block: Option<usize>,
        indices: &[usize],
        discard: &mut [u8],
        mut check: impl FnMut() -> io::Result<()>,
        mut visit: impl FnMut(usize, &ArchiveEntry, &mut dyn Read) -> io::Result<()>,
    ) -> Result<()> {
        let Some(block) = block else {
            for &index in indices {
                check()?;
                visit(index, &self.archive.files[index], &mut io::empty())?;
            }
            return Ok(());
        };
        let mut input = BufReader::with_capacity(256 * 1024, self.input.clone());
        let mut index = self.archive.stream_map.block_first_file_index[block];
        let mut pending = indices.iter().copied().peekable();
        let mut callback_error = None;
        let decode = BlockDecoder::new(1, block, &self.archive, &self.password, &mut input)
            .for_each_entries(&mut |entry, reader| {
                let result = (|| -> io::Result<bool> {
                    check()?;
                    if pending.peek() == Some(&index) {
                        visit(index, entry, reader)?;
                        pending.next();
                    } else {
                        loop {
                            check()?;
                            if reader.read(discard)? == 0 {
                                break;
                            }
                        }
                    }
                    index += 1;
                    Ok(pending.peek().is_some())
                })();
                match result {
                    Ok(keep_reading) => Ok(keep_reading),
                    Err(error) => {
                        callback_error = Some(error);
                        Ok(false)
                    }
                }
            });
        if let Some(error) = callback_error {
            return Err(error.into());
        }
        decode?;
        ensure!(
            pending.peek().is_none(),
            "7z block ended before its selected entries"
        );
        Ok(())
    }

    pub fn copy_index(
        &self,
        index: usize,
        output: &mut impl io::Write,
        mut check: impl FnMut() -> io::Result<()>,
    ) -> Result<()> {
        let entry = &self.archive.files[index];
        ensure!(
            !entry.is_directory && !entry.is_anti_item,
            "7z member is not a regular file"
        );
        let block = entry
            .has_stream
            .then_some(self.archive.stream_map.file_block_index[index])
            .flatten();
        let (mut discard, mut buffer) = (vec![0; 256 * 1024], vec![0; 256 * 1024]);
        let check = std::cell::RefCell::new(&mut check);
        self.read_group(
            block,
            &[index],
            &mut discard,
            || (check.borrow_mut())(),
            |_, entry, input| {
                let mut bytes = 0u64;
                loop {
                    (check.borrow_mut())()?;
                    let count = input.read(&mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    output.write_all(&buffer[..count])?;
                    bytes += count as u64;
                }
                if bytes != entry.size {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Truncated 7z member",
                    ));
                }
                Ok(())
            },
        )
    }

    pub fn read_texts(
        &self,
        names: impl IntoIterator<Item = String>,
        deadline: Option<Instant>,
    ) -> HashMap<String, String> {
        let indices = names
            .into_iter()
            .filter_map(|name| self.index(&name.replace('\\', "/")))
            .filter(|&index| {
                let entry = &self.archive.files[index];
                entry.size <= 1024 * 1024 && !entry.is_directory && !entry.is_anti_item
            });
        let mut texts = HashMap::new();
        let mut discard = vec![0; 256 * 1024];
        for (block, entries) in self.groups(indices) {
            let _ = self.read_group(
                block,
                &entries,
                &mut discard,
                || {
                    if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "7z metadata read timed out",
                        ));
                    }
                    Ok(())
                },
                |_, entry, input| {
                    let mut bytes = Vec::with_capacity(entry.size as usize);
                    input.read_to_end(&mut bytes)?;
                    if bytes.len() as u64 == entry.size {
                        if let Ok(text) = String::from_utf8(bytes) {
                            texts.insert(entry.name.replace('\\', "/"), text);
                        }
                    }
                    Ok(())
                },
            );
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                break;
            }
        }
        texts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sevenz_rust2::{ArchiveWriter, EncoderConfiguration, EncoderMethod, SourceReader};

    #[test]
    fn solid_7z_streams_a_late_file_and_skips_unrelated_blocks() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("solid.7z");
        let mut writer = ArchiveWriter::create(&path)?;
        let large = vec![0x7b; 2 * 1024 * 1024];
        writer.push_archive_entries(
            vec![
                ArchiveEntry::new_file("large.bin"),
                ArchiveEntry::new_file("Addon/version.txt"),
                ArchiveEntry::new_file("Addon/readme.txt"),
            ],
            vec![
                SourceReader::new(large.as_slice()),
                SourceReader::new(b"v1.2.3".as_slice()),
                SourceReader::new(b"readme".as_slice()),
            ],
        )?;
        writer.push_archive_entry(
            ArchiveEntry::new_file("other.bin"),
            Some(b"other".as_slice()),
        )?;
        writer.finish()?;

        let package = SevenzPackage::open(&path, None)?;
        assert!(package.archive.is_solid);
        let texts = package.read_texts(
            ["Addon/version.txt".into(), "Addon/readme.txt".into()],
            None,
        );
        assert_eq!(texts["Addon/version.txt"], "v1.2.3");
        assert_eq!(texts["Addon/readme.txt"], "readme");
        let index = package.index("Addon/version.txt").unwrap();
        let mut output = Vec::new();
        let mut checks = 0;
        let result = package.copy_index(index, &mut output, || {
            checks += 1;
            if checks == 4 {
                return Err(io::Error::other("Cancelled by user"));
            }
            Ok(())
        });
        assert!(result.unwrap_err().to_string().contains("Cancelled"));
        assert!(output.is_empty());
        Ok(())
    }

    #[test]
    fn stored_7z_member_exposes_only_its_byte_range() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("copy.7z");
        let mut writer = ArchiveWriter::create(&path)?;
        writer.set_content_methods(vec![EncoderConfiguration::new(EncoderMethod::COPY)]);
        writer.push_archive_entry(
            ArchiveEntry::new_file("child.zip"),
            Some(b"zip payload".as_slice()),
        )?;
        writer.push_archive_entry(
            ArchiveEntry::new_file("other.bin"),
            Some(b"other".as_slice()),
        )?;
        writer.finish()?;
        let package = SevenzPackage::open(&path, None)?;
        let index = package.index("child.zip").unwrap();
        let mut member = package.stored_member(index)?.unwrap();
        let mut bytes = Vec::new();
        member.read_to_end(&mut bytes)?;
        assert_eq!(bytes, b"zip payload");
        Ok(())
    }
}
