use std::ffi::CString;
use std::io;
use std::path::Path;
use std::ptr;

use anyhow::{bail, Context, Result};
use unrar_sys as native;

pub(crate) struct RarStream(*const native::Handle);

pub(crate) struct RarMember {
    pub name: String,
    pub size: u64,
    pub directory: bool,
    pub redirected: bool,
}

struct Callback<'a> {
    write: &'a mut dyn FnMut(&[u8]) -> io::Result<()>,
    error: Option<io::Error>,
}

extern "C" fn rar_open_callback(
    message: native::UINT,
    data: native::LPARAM,
    buffer: native::LPARAM,
    length: native::LPARAM,
) -> std::ffi::c_int {
    if data != 0 && buffer != 0 && length > 0 {
        // The open operation borrows the password synchronously.
        let password = unsafe { &*(data as *const &str) };
        if message == native::UCM_NEEDPASSWORD {
            let mut bytes = password
                .as_bytes()
                .iter()
                .copied()
                .chain(std::iter::once(0));
            if password.len() >= length as usize {
                return -1;
            }
            for (index, byte) in bytes.by_ref().enumerate() {
                unsafe { (buffer as *mut u8).add(index).write(byte) };
            }
            return 1;
        }
        if message == native::UCM_NEEDPASSWORDW {
            #[cfg(windows)]
            let units = password.encode_utf16().chain(std::iter::once(0));
            #[cfg(unix)]
            let units = password
                .chars()
                .map(|character| character as native::WCHAR)
                .chain(std::iter::once(0));
            for (index, unit) in units.enumerate() {
                if index >= length as usize {
                    return -1;
                }
                unsafe { (buffer as *mut native::WCHAR).add(index).write(unit) };
            }
            return 1;
        }
    }
    rar_callback(message, 0, buffer, length)
}

extern "C" fn rar_callback(
    message: native::UINT,
    data: native::LPARAM,
    buffer: native::LPARAM,
    length: native::LPARAM,
) -> std::ffi::c_int {
    match message {
        native::UCM_PROCESSDATA if data != 0 && buffer != 0 && length > 0 => {
            // UnRAR calls synchronously with the callback state and a valid decoded byte slice.
            let state = unsafe { &mut *(data as *mut Callback<'_>) };
            let bytes = unsafe { std::slice::from_raw_parts(buffer as *const u8, length as usize) };
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (state.write)(bytes)));
            match result {
                Ok(Ok(())) => 1,
                Ok(Err(error)) => {
                    state.error = Some(error);
                    -1
                }
                Err(_) => {
                    state.error = Some(io::Error::other("RAR output callback failed"));
                    -1
                }
            }
        }
        native::UCM_CHANGEVOLUME | native::UCM_CHANGEVOLUMEW if length == native::RAR_VOL_ASK => -1,
        native::UCM_NEEDPASSWORD | native::UCM_NEEDPASSWORDW => -1,
        _ => 1,
    }
}

impl RarStream {
    pub fn open(path: &Path, password: Option<&str>) -> Result<Self> {
        let password_c = password
            .map(CString::new)
            .transpose()
            .context("Invalid RAR password")?;
        #[cfg(any(target_os = "linux", target_os = "netbsd"))]
        let path = {
            use std::os::unix::ffi::OsStrExt;
            CString::new(path.as_os_str().as_bytes())?
        };
        #[cfg(windows)]
        let path = {
            use std::os::windows::ffi::OsStrExt;
            let mut path: Vec<_> = path.as_os_str().encode_wide().collect();
            anyhow::ensure!(!path.contains(&0), "Invalid RAR path");
            path.push(0);
            path
        };
        #[cfg(all(unix, not(any(target_os = "linux", target_os = "netbsd"))))]
        let path = path
            .to_str()
            .context("Invalid RAR path")?
            .chars()
            .map(|character| character as native::WCHAR)
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();

        let mut data = native::OpenArchiveDataEx::new(path.as_ptr(), native::RAR_OM_EXTRACT);
        data.callback = Some(rar_open_callback);
        data.user_data = password
            .as_ref()
            .map_or(0, |password| password as *const &str as native::LPARAM);
        let handle = unsafe { native::RAROpenArchiveEx(&mut data) };
        if handle.is_null() {
            bail!(
                "Failed to open RAR archive (UnRAR error {})",
                data.open_result
            );
        }
        let reader = Self(handle);
        unsafe { native::RARSetCallback(handle, Some(rar_callback), 0) };
        reader.check(data.open_result as std::ffi::c_int, "open archive")?;
        if let Some(password) = password_c {
            unsafe { native::RARSetPassword(handle, password.as_ptr()) };
        }
        Ok(reader)
    }

    pub fn next(&mut self) -> Result<Option<RarMember>> {
        let mut header = native::HeaderDataEx::default();
        let status = unsafe { native::RARReadHeaderEx(self.0, &mut header) };
        if status == native::ERAR_END_ARCHIVE {
            return Ok(None);
        }
        self.check(status, "read header")?;
        let end = header
            .filename_w
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(header.filename_w.len());
        let name = &header.filename_w[..end];
        #[cfg(windows)]
        let name = String::from_utf16_lossy(name);
        #[cfg(unix)]
        let name: String = name
            .iter()
            .map(|&unit| char::from_u32(unit as u32).unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect();
        Ok(Some(RarMember {
            name: name.replace('\\', "/"),
            size: (u64::from(header.unp_size_high) << 32) | u64::from(header.unp_size),
            directory: header.flags & native::RHDF_DIRECTORY != 0,
            redirected: header.redir_type != 0
                || (header.host_os == 3 && header.file_attr & 0o170000 == 0o120000),
        }))
    }

    pub fn skip(&mut self) -> Result<()> {
        let status =
            unsafe { native::RARProcessFileW(self.0, native::RAR_SKIP, ptr::null(), ptr::null()) };
        self.check(status, "skip entry")
    }

    pub fn read(&mut self, mut write: impl FnMut(&[u8]) -> io::Result<()>) -> Result<()> {
        let mut callback = Callback {
            write: &mut write,
            error: None,
        };
        // Test mode validates the RAR stream and delivers decoded bytes without letting the
        // library create filesystem entries. The synchronous callback borrows this stack state.
        let status = unsafe {
            native::RARSetCallback(
                self.0,
                Some(rar_callback),
                &mut callback as *mut _ as native::LPARAM,
            );
            let status =
                native::RARProcessFileW(self.0, native::RAR_TEST, ptr::null(), ptr::null());
            native::RARSetCallback(self.0, Some(rar_callback), 0);
            status
        };
        if let Some(error) = callback.error {
            return Err(error.into());
        }
        self.check(status, "decode entry")
    }

    fn check(&self, code: std::ffi::c_int, operation: &str) -> Result<()> {
        match code {
            native::ERAR_SUCCESS => Ok(()),
            native::ERAR_MISSING_PASSWORD => bail!("Password required for RAR archive"),
            native::ERAR_BAD_PASSWORD => bail!("Wrong password for RAR archive"),
            _ => bail!("Failed to {operation} in RAR archive (UnRAR error {code})"),
        }
    }
}

impl Drop for RarStream {
    fn drop(&mut self) {
        unsafe { native::RARCloseArchive(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_rar_fixture_streams_selected_member_when_supplied() -> Result<()> {
        let Some(path) = std::env::var_os("XFAST_TEST_RAR") else {
            return Ok(());
        };
        let name = std::env::var("XFAST_TEST_RAR_MEMBER")?;
        let password = std::env::var("XFAST_TEST_RAR_PASSWORD").ok();
        if password.is_some() {
            let wrong = RarStream::open(Path::new(&path), Some("incorrect"));
            assert!(wrong.is_err());
        }
        let mut stream = RarStream::open(Path::new(&path), password.as_deref())?;
        while let Some(member) = stream.next()? {
            if member.name == name {
                let mut output = Vec::new();
                assert!(stream
                    .read(|_| Err(io::Error::other("Cancelled by user")))
                    .is_err());
                let mut retry = RarStream::open(Path::new(&path), password.as_deref())?;
                while let Some(member) = retry.next()? {
                    if member.name == name {
                        retry.read(|bytes| {
                            output.extend_from_slice(bytes);
                            Ok(())
                        })?;
                        assert_eq!(output, b"selected RAR entry");
                        return Ok(());
                    }
                    retry.skip()?;
                }
                break;
            }
            stream.skip()?;
        }
        bail!("Missing RAR test member: {name}")
    }
}
