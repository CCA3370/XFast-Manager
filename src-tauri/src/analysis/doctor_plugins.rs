use serde::Serialize;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const ENTRY_LIMIT: usize = 60_000;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PluginInspection {
    pub(super) checked: usize,
    pub(super) problems: Vec<String>,
    pub(super) errors: Vec<String>,
    pub(super) capped: bool,
}

#[derive(Clone, Copy)]
enum Platform {
    Windows,
    Linux,
    Mac,
}

impl Platform {
    fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Linux
        }
    }

    fn layout(self) -> (&'static str, &'static str) {
        match self {
            Self::Windows => ("win_x64", "win.xpl"),
            Self::Linux => ("lin_x64", "lin.xpl"),
            Self::Mac => ("mac_x64", "mac.xpl"),
        }
    }
}

pub(super) fn inspect(root: &Path, scenery: Option<&[PathBuf]>) -> PluginInspection {
    scan(root, scenery, Platform::current(), ENTRY_LIMIT)
}

#[cfg(test)]
pub(super) fn test_layout() -> (&'static str, &'static str) {
    Platform::current().layout()
}

fn scan(
    root: &Path,
    scenery: Option<&[PathBuf]>,
    platform: Platform,
    remaining: usize,
) -> PluginInspection {
    let mut scanner = Scanner {
        root,
        platform,
        remaining,
        result: PluginInspection::default(),
    };
    scanner.container(&root.join("Resources/plugins"), true);
    scanner.aircraft();
    if let Some(scenery) = scenery {
        for pack in scenery {
            if scanner.result.capped {
                break;
            }
            scanner.container(&pack.join("plugins"), false);
        }
    } else {
        let scenery = root.join("Custom Scenery");
        match fs::metadata(&scenery) {
            Ok(_) => scanner.error(
                &scenery.join("scenery_packs.ini"),
                "enabled scenery plugin folders could not be determined",
            ),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => scanner.error(&scenery, error),
        }
    }
    scanner.result.problems.sort();
    scanner.result.errors.sort();
    scanner.result
}

struct Scanner<'a> {
    root: &'a Path,
    platform: Platform,
    remaining: usize,
    result: PluginInspection,
}

impl Scanner<'_> {
    fn describe(&self, path: &Path, detail: impl std::fmt::Display) -> String {
        format!(
            "{}: {detail}",
            path.strip_prefix(self.root).unwrap_or(path).display()
        )
    }

    fn error(&mut self, path: &Path, error: impl std::fmt::Display) {
        self.result.errors.push(self.describe(path, error));
    }

    // A truncated listing cannot establish which entries belong to this container.
    fn entries(&mut self, path: &Path) -> Option<Vec<fs::DirEntry>> {
        if self.result.capped {
            return None;
        }
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Some(Vec::new()),
            Err(error) => {
                self.error(path, error);
                return None;
            }
        };
        let mut listed = Vec::new();
        for entry in entries {
            if self.remaining == 0 {
                self.result.capped = true;
                return None;
            }
            self.remaining -= 1;
            match entry {
                Ok(entry) => listed.push(entry),
                Err(error) => {
                    self.error(path, error);
                    return None;
                }
            }
        }
        Some(listed)
    }

    fn entry_type(&mut self, entry: &fs::DirEntry) -> Option<fs::FileType> {
        match entry.file_type().and_then(|kind| {
            if kind.is_symlink() {
                fs::metadata(entry.path()).map(|meta| meta.file_type())
            } else {
                Ok(kind)
            }
        }) {
            Ok(kind) => Some(kind),
            Err(error) => {
                self.error(&entry.path(), error);
                None
            }
        }
    }

    fn container(&mut self, path: &Path, allow_loose_files: bool) {
        let Some(entries) = self.entries(path) else {
            return;
        };
        for entry in entries.into_iter().filter(visible) {
            if self.result.capped {
                break;
            }
            let Some(kind) = self.entry_type(&entry) else {
                continue;
            };
            if kind.is_dir() {
                self.bundle(&entry.path());
            } else if allow_loose_files && kind.is_file() && has_extension(&entry.path(), "xpl") {
                self.check(&entry.path());
            }
        }
    }

    fn bundle(&mut self, path: &Path) {
        let (platform_dir, legacy_name) = self.platform.layout();
        let Some(name) = path.file_name() else {
            return;
        };
        let mut binary_name = name.to_os_string();
        binary_name.push(".xpl");
        // XPLM looks for the bundle's own name in the platform directory first.
        // Other .xpl files beside it can be auxiliary or alternate binaries.
        for candidate in [
            path.join(platform_dir).join(binary_name),
            path.join("64").join(legacy_name),
            path.join(legacy_name),
        ] {
            match fs::metadata(&candidate) {
                Ok(meta) if meta.is_file() => return self.check(&candidate),
                Ok(_) => return self.error(&candidate, "plugin entry is not a regular file"),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    if fs::symlink_metadata(&candidate)
                        .is_ok_and(|meta| meta.file_type().is_symlink())
                    {
                        return self
                            .error(&candidate, "plugin entry link target could not be read");
                    }
                }
                Err(error) => return self.error(&candidate, error),
            }
        }
    }

    fn aircraft(&mut self) {
        let mut pending = vec![self.root.join("Aircraft")];
        let mut visited = HashSet::new();
        while let Some(path) = pending.pop() {
            if self.result.capped {
                break;
            }
            let canonical = match path.canonicalize() {
                Ok(canonical) => canonical,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => {
                    self.error(&path, error);
                    continue;
                }
            };
            if !visited.insert(canonical) {
                continue;
            }
            let Some(entries) = self.entries(&path) else {
                continue;
            };
            let mut has_aircraft = false;
            let mut enabled = false;
            for entry in entries.iter().filter(|entry| visible(entry)) {
                let path = entry.path();
                let acf = has_extension(&path, "acf");
                if (acf || has_extension(&path, "xfma"))
                    && self.entry_type(entry).is_some_and(|kind| kind.is_file())
                {
                    has_aircraft = true;
                    if acf {
                        enabled = true;
                        break;
                    }
                }
            }
            if enabled {
                self.container(&path.join("plugins"), false);
            } else if !has_aircraft {
                pending.extend(entries.into_iter().filter(visible).filter_map(|entry| {
                    if entry.file_name().eq_ignore_ascii_case("plugins") {
                        return None;
                    }
                    self.entry_type(&entry)
                        .filter(|kind| kind.is_dir())
                        .map(|_| entry.path())
                }));
            }
        }
    }

    fn check(&mut self, path: &Path) {
        match binary_problem(path, self.platform) {
            Ok(problem) => {
                self.result.checked += 1;
                if let Some(problem) = problem {
                    self.result.problems.push(self.describe(path, problem));
                }
            }
            Err(error) => self.error(path, error),
        }
    }
}

fn visible(entry: &fs::DirEntry) -> bool {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    !name.starts_with('.') && !name.to_ascii_lowercase().ends_with(".disabled")
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
}

fn read_at<const N: usize>(file: &mut File, offset: u64) -> io::Result<[u8; N]> {
    let mut header = [0; N];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut header)?;
    Ok(header)
}

pub(super) fn host_binary_problem(path: &Path) -> io::Result<Option<&'static str>> {
    binary_problem(path, Platform::current())
}

fn binary_problem(path: &Path, platform: Platform) -> io::Result<Option<&'static str>> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    if size == 0 {
        return Ok(Some("empty executable binary"));
    }
    let result = match platform {
        Platform::Windows => pe_problem(&mut file, size),
        Platform::Linux => elf_problem(&mut file),
        Platform::Mac => macho_problem(&mut file, size),
    };
    match result {
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
            Ok(Some("truncated executable header"))
        }
        result => result,
    }
}

fn pe_problem(file: &mut File, size: u64) -> io::Result<Option<&'static str>> {
    let dos = read_at::<64>(file, 0)?;
    if &dos[..2] != b"MZ" {
        return Ok(Some("invalid PE signature"));
    }
    let offset = u32::from_le_bytes(dos[60..64].try_into().unwrap()) as u64;
    if offset < 64 || offset + 26 > size {
        return Ok(Some("invalid PE header offset"));
    }
    let pe = read_at::<26>(file, offset)?;
    if &pe[..4] != b"PE\0\0" {
        return Ok(Some("invalid PE signature"));
    }
    let machine = u16::from_le_bytes([pe[4], pe[5]]);
    let magic = u16::from_le_bytes([pe[24], pe[25]]);
    if machine != 0x8664 || magic != 0x20b {
        return Ok(Some("expected an x86-64 PE32+ entry"));
    }
    let optional_size = u16::from_le_bytes([pe[20], pe[21]]) as u64;
    if optional_size < 112 || offset + 24 + optional_size > size {
        return Ok(Some("incomplete PE optional header"));
    }
    Ok(None)
}

fn elf_problem(file: &mut File) -> io::Result<Option<&'static str>> {
    let header = read_at::<64>(file, 0)?;
    if &header[..4] != b"\x7fELF" {
        return Ok(Some("invalid ELF signature"));
    }
    if header[4..6] != [2, 1] || u16::from_le_bytes([header[18], header[19]]) != 62 {
        return Ok(Some("expected a little-endian x86-64 ELF entry"));
    }
    Ok(None)
}

fn macho_problem(file: &mut File, size: u64) -> io::Result<Option<&'static str>> {
    let header = read_at::<8>(file, 0)?;
    let magic = u32::from_be_bytes(header[..4].try_into().unwrap());
    match magic {
        0xcffaedfe if size >= 32 => {
            // The manager's process architecture does not tell us whether X-Plane
            // will run natively or under Rosetta. Accept both SDK architectures.
            let cpu = u32::from_le_bytes(header[4..8].try_into().unwrap());
            Ok((!matches!(cpu, 0x0100_0007 | 0x0100_000c))
                .then_some("unsupported 64-bit Mach-O architecture"))
        }
        0xcafebabe | 0xcafebabf => {
            let count = u32::from_be_bytes(header[4..8].try_into().unwrap()) as u64;
            let wide = magic == 0xcafebabf;
            let stride = if wide { 32 } else { 20 };
            let end = 8 + stride * count;
            if end > size || count == 0 || count > 128 {
                return Ok(Some("invalid universal Mach-O architecture table"));
            }
            for index in 0..count {
                let entry = read_at::<20>(file, 8 + stride * index)?;
                let cpu = u32::from_be_bytes(entry[..4].try_into().unwrap());
                if !matches!(cpu, 0x0100_0007 | 0x0100_000c) {
                    continue;
                }
                let (offset, length) = if wide {
                    let bytes = read_at::<16>(file, 16 + stride * index)?;
                    (
                        u64::from_be_bytes(bytes[..8].try_into().unwrap()),
                        u64::from_be_bytes(bytes[8..].try_into().unwrap()),
                    )
                } else {
                    (
                        u32::from_be_bytes(entry[8..12].try_into().unwrap()) as u64,
                        u32::from_be_bytes(entry[12..16].try_into().unwrap()) as u64,
                    )
                };
                if offset >= end && length >= 32 && offset <= size && length <= size - offset {
                    let slice = read_at::<8>(file, offset)?;
                    if slice[..4] == [0xcf, 0xfa, 0xed, 0xfe]
                        && u32::from_le_bytes(slice[4..].try_into().unwrap()) == cpu
                    {
                        return Ok(None);
                    }
                }
            }
            Ok(Some("no supported 64-bit slice in universal Mach-O"))
        }
        _ => Ok(Some("invalid or unsupported 64-bit Mach-O header")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(root: &Path, relative: &str, bytes: &[u8]) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn checks_the_named_entry_and_ignores_alternate_binaries() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        put(root, "Resources/plugins/loose.xpl", b"");
        put(
            root,
            "Resources/plugins/Selected/win_x64/Selected.xpl",
            b"bad",
        );
        put(
            root,
            "Resources/plugins/Selected/win_x64/Selected.alt.xpl",
            b"",
        );
        fs::create_dir_all(root.join("Resources/plugins/Unreadable/win_x64/Unreadable.xpl"))
            .unwrap();
        put(root, "Resources/plugins/Unreadable/64/win.xpl", b"");
        put(root, "Resources/plugins/Other/lin_x64/Other.xpl", b"");
        put(root, "Resources/plugins/Old/32/win.xpl", b"");
        put(
            root,
            "Resources/plugins/Disabled/win_x64/Disabled.xfmp",
            b"",
        );

        let result = scan(root, Some(&[]), Platform::Windows, ENTRY_LIMIT);
        assert_eq!(result.checked, 2);
        assert_eq!(result.problems.len(), 2);
        assert!(result
            .problems
            .iter()
            .any(|problem| problem.contains("Selected.xpl")));
        assert!(result
            .problems
            .iter()
            .any(|problem| problem.contains("loose.xpl")));
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("Unreadable.xpl"));
    }

    #[test]
    fn a_capped_inventory_does_not_claim_that_entries_were_checked() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        put(root, "Resources/plugins/Test/win_x64/Test.xpl", b"");
        put(root, "Resources/plugins/Test/64/win.xpl", b"");
        let result = scan(root, Some(&[]), Platform::Windows, 0);
        assert!(result.capped);
        assert_eq!(result.checked, 0);
        assert!(result.problems.is_empty());
    }

    #[test]
    fn checks_elf_machine_and_mach_o_architecture_without_the_host_loader() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let mut elf = [0; 64];
        elf[..6].copy_from_slice(b"\x7fELF\x02\x01");
        elf[18] = 62;
        put(root, "valid-elf.xpl", &elf);
        elf[18] = 183;
        put(root, "arm-elf.xpl", &elf);
        assert_eq!(
            binary_problem(&root.join("valid-elf.xpl"), Platform::Linux).unwrap(),
            None
        );
        assert!(binary_problem(&root.join("arm-elf.xpl"), Platform::Linux)
            .unwrap()
            .is_some());

        let mut universal = [0; 64];
        universal[..8].copy_from_slice(b"\xca\xfe\xba\xbe\0\0\0\x01");
        universal[8..12].copy_from_slice(&0x0100_000cu32.to_be_bytes());
        universal[16..20].copy_from_slice(&32u32.to_be_bytes());
        universal[20..24].copy_from_slice(&32u32.to_be_bytes());
        universal[32..36].copy_from_slice(b"\xcf\xfa\xed\xfe");
        universal[36..40].copy_from_slice(&0x0100_000cu32.to_le_bytes());
        put(root, "universal.xpl", &universal);
        put(root, "truncated.xpl", &universal[..40]);
        assert_eq!(
            binary_problem(&root.join("universal.xpl"), Platform::Mac).unwrap(),
            None
        );
        assert!(binary_problem(&root.join("truncated.xpl"), Platform::Mac)
            .unwrap()
            .is_some());
    }
}
