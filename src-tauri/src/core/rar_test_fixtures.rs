//! Small, uncompressed RAR fixtures generated for tests, without an external archiver.
//!
//! Layouts follow RARLAB's RAR 3.93 technical note (the RAR4 layout) and
//! <https://www.rarlab.com/technote.htm> (RAR5). These fixtures have ordinary files,
//! a main header and an end marker, with valid header and payload CRCs. They do not
//! contain encryption, split volumes, solid compression or Quick Open records.

#[derive(Clone, Copy, Debug)]
pub(crate) enum RarVersion {
    Rar4,
    Rar5,
}

pub(crate) struct RarFixture {
    pub bytes: Vec<u8>,
    /// Start of each file header, including its CRC. Flip a byte here to damage
    /// only that header, without changing its name, length or payload.
    pub header_offsets: Vec<usize>,
    /// First payload byte of each file, immediately after its complete header.
    pub data_offsets: Vec<usize>,
}

/// Build a complete archive containing the supplied regular files in order.
/// Tests can corrupt or truncate the returned bytes at a known member boundary.
pub(crate) fn stored_rar(version: RarVersion, entries: &[(&str, &[u8])]) -> RarFixture {
    let mut fixture = RarFixture {
        bytes: Vec::new(),
        header_offsets: Vec::with_capacity(entries.len()),
        data_offsets: Vec::with_capacity(entries.len()),
    };

    match version {
        RarVersion::Rar4 => {
            fixture.bytes.extend_from_slice(b"Rar!\x1a\x07\x00");
            // MAIN_HEAD: no flags, followed by six reserved zero bytes.
            rar4_header(&mut fixture.bytes, 0x73, 0, &[0; 6]);
        }
        RarVersion::Rar5 => {
            fixture.bytes.extend_from_slice(b"Rar!\x1a\x07\x01\x00");
            // Main header type, common flags, archive flags.
            rar5_header(&mut fixture.bytes, &[1, 0, 0]);
        }
    }

    for &(name, data) in entries {
        assert!(!name.is_empty() && !name.contains('\0'));
        fixture.header_offsets.push(fixture.bytes.len());
        match version {
            RarVersion::Rar4 => {
                // OEM names are deliberately ASCII; encoded RAR4 Unicode names
                // need a different wire layout and are outside these fixtures.
                assert!(name.is_ascii(), "RAR4 test names must be ASCII");
                let size = u32::try_from(data.len()).expect("RAR4 test payload exceeds 4 GiB");
                let name_size = u16::try_from(name.len()).expect("RAR4 test name is too long");
                let mut fields = Vec::new();
                fields.extend_from_slice(&size.to_le_bytes()); // Packed size / ADD_SIZE.
                fields.extend_from_slice(&size.to_le_bytes()); // Unpacked size.
                fields.push(2); // Win32 host OS.
                fields.extend_from_slice(&crc32fast::hash(data).to_le_bytes());
                fields.extend_from_slice(&0x0021_0000u32.to_le_bytes()); // 1980-01-01 DOS time.
                fields.push(29); // UnRAR 2.9 or newer.
                fields.push(0x30); // Store without compression.
                fields.extend_from_slice(&name_size.to_le_bytes());
                fields.extend_from_slice(&0x20u32.to_le_bytes()); // Regular archive file.
                fields.extend_from_slice(name.as_bytes());
                rar4_header(&mut fixture.bytes, 0x74, 0x8000, &fields); // LONG_BLOCK.
            }
            RarVersion::Rar5 => {
                let mut header = vec![2, 2]; // File header, data area present.
                vint(&mut header, data.len() as u64); // Packed size.
                vint(&mut header, 4); // File flags: data CRC32 present.
                vint(&mut header, data.len() as u64); // Unpacked size.
                vint(&mut header, 0x20); // Windows regular archive file attributes.
                header.extend_from_slice(&crc32fast::hash(data).to_le_bytes());
                vint(&mut header, 0); // RAR5, non-solid, stored, minimum dictionary.
                vint(&mut header, 0); // Windows host OS.
                vint(&mut header, name.len() as u64);
                header.extend_from_slice(name.as_bytes());
                rar5_header(&mut fixture.bytes, &header);
            }
        }
        fixture.data_offsets.push(fixture.bytes.len());
        fixture.bytes.extend_from_slice(data);
    }

    match version {
        RarVersion::Rar4 => rar4_header(&mut fixture.bytes, 0x7b, 0x4000, &[]),
        RarVersion::Rar5 => rar5_header(&mut fixture.bytes, &[5, 0, 0]),
    }
    fixture
}

fn rar4_header(output: &mut Vec<u8>, kind: u8, flags: u16, fields: &[u8]) {
    let size = u16::try_from(7 + fields.len()).expect("RAR4 test header is too long");
    let mut header = vec![kind];
    header.extend_from_slice(&flags.to_le_bytes());
    header.extend_from_slice(&size.to_le_bytes());
    header.extend_from_slice(fields);
    // RAR4 uses the low 16 bits of the ordinary CRC32, excluding the CRC field.
    output.extend_from_slice(&(crc32fast::hash(&header) as u16).to_le_bytes());
    output.extend_from_slice(&header);
}

fn rar5_header(output: &mut Vec<u8>, body: &[u8]) {
    let mut header = Vec::new();
    // Header size counts only the body; the CRC covers this size vint as well.
    vint(&mut header, body.len() as u64);
    header.extend_from_slice(body);
    output.extend_from_slice(&crc32fast::hash(&header).to_le_bytes());
    output.extend_from_slice(&header);
}

fn vint(output: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        output.push(byte | if value == 0 { 0 } else { 0x80 });
        if value == 0 {
            break;
        }
    }
}
