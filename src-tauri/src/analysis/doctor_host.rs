//! Host checks for X-Plane and XFast Manager. Installation probes do not create files.
use serde::Serialize;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSpace {
    available_bytes: Option<u64>,
    total_bytes: Option<u64>,
    error: Option<String>,
}

pub fn volume_space(path: &Path) -> VolumeSpace {
    match fs::metadata(path).and_then(|_| fs2::statvfs(path)) {
        Ok(stats) => VolumeSpace {
            available_bytes: Some(stats.available_space()),
            total_bytes: Some(stats.total_space()),
            error: None,
        },
        Err(error) => VolumeSpace {
            available_bytes: None,
            total_bytes: None,
            error: Some(format!("{}: {error}", path.display())),
        },
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryAccess {
    blocked: Vec<String>,
    errors: Vec<String>,
}

pub fn directory_access(root: &Path) -> DirectoryAccess {
    let mut report = DirectoryAccess::default();
    for name in [
        "",
        "Output",
        "Output/preferences",
        "Custom Data",
        "Custom Scenery",
        "Aircraft",
        "Resources/plugins",
    ] {
        let path = root.join(name);
        let label = if name.is_empty() {
            "X-Plane root"
        } else {
            name
        };
        match fs::metadata(&path) {
            Ok(meta) if !meta.is_dir() => {
                report.blocked.push(format!("{label}: not a directory"));
                continue;
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound && !name.is_empty() => continue,
            Err(error) => {
                report.errors.push(format!("{label}: {error}"));
                continue;
            }
        }
        match check_add_file_permission(&path) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::ReadOnlyFilesystem
                ) =>
            {
                report.blocked.push(format!("{label}: {error}"));
            }
            Err(error) => report.errors.push(format!("{label}: {error}")),
        }
    }
    report
}

#[cfg(windows)]
fn check_add_file_permission(path: &Path) -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    // FILE_ADD_FILE, shared access, and FILE_FLAG_BACKUP_SEMANTICS for a directory handle.
    fs::OpenOptions::new()
        .access_mode(0x0002)
        .share_mode(0x0007)
        .custom_flags(0x0200_0000)
        .open(path)
        .map(drop)
}

#[cfg(unix)]
fn check_add_file_permission(path: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let path = CString::new(path.as_os_str().as_bytes())?;
    // The process needs both write and search access to add an entry.
    if unsafe {
        libc::faccessat(
            libc::AT_FDCWD,
            path.as_ptr(),
            libc::W_OK | libc::X_OK,
            libc::AT_EACCESS,
        )
    } == 0
    {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "state"
)]
pub enum VulkanRuntime {
    Available {
        version: String,
        meets_minimum: bool,
    },
    Missing {
        detail: String,
    },
    Unavailable {
        detail: String,
    },
}

#[cfg(any(windows, target_os = "linux"))]
type QueryVersion = unsafe extern "system" fn(*mut u32) -> i32;

#[cfg(any(windows, target_os = "linux"))]
fn describe_version(query: Option<QueryVersion>) -> VulkanRuntime {
    // A loader without vkEnumerateInstanceVersion implements Vulkan 1.0.
    let mut encoded = 1 << 22;
    if let Some(query) = query {
        let status = unsafe { query(&mut encoded) };
        if status != 0 {
            return VulkanRuntime::Unavailable {
                detail: format!("vkEnumerateInstanceVersion returned {status}"),
            };
        }
    }
    let variant = encoded >> 29;
    let major = (encoded >> 22) & 0x7f;
    let minor = (encoded >> 12) & 0x3ff;
    let patch = encoded & 0xfff;
    VulkanRuntime::Available {
        version: format!("{major}.{minor}.{patch}"),
        meets_minimum: variant == 0 && (major, minor) >= (1, 3),
    }
}

#[cfg(windows)]
pub fn vulkan_runtime() -> Option<VulkanRuntime> {
    use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryExW};

    let name: Vec<u16> = "vulkan-1.dll".encode_utf16().chain([0]).collect();
    // LOAD_LIBRARY_SEARCH_SYSTEM32 confines the probe to the Windows loader.
    let library = unsafe { LoadLibraryExW(name.as_ptr(), std::ptr::null_mut(), 0x0000_0800) };
    if library.is_null() {
        let error = io::Error::last_os_error();
        let detail = format!("vulkan-1.dll: {error}");
        return Some(if error.raw_os_error() == Some(126) {
            VulkanRuntime::Missing { detail }
        } else {
            VulkanRuntime::Unavailable { detail }
        });
    }
    let address = unsafe { GetProcAddress(library, c"vkEnumerateInstanceVersion".as_ptr()) };
    let query =
        (!address.is_null()).then(|| unsafe { std::mem::transmute::<_, QueryVersion>(address) });
    let report = describe_version(query);
    unsafe { FreeLibrary(library) };
    Some(report)
}

#[cfg(target_os = "linux")]
pub fn vulkan_runtime() -> Option<VulkanRuntime> {
    use std::ffi::CStr;
    let library = unsafe {
        libc::dlopen(
            c"libvulkan.so.1".as_ptr(),
            libc::RTLD_NOW | libc::RTLD_LOCAL,
        )
    };
    if library.is_null() {
        let message = unsafe { libc::dlerror() };
        let detail = if message.is_null() {
            "libvulkan.so.1 could not be loaded".into()
        } else {
            unsafe { CStr::from_ptr(message) }
                .to_string_lossy()
                .into_owned()
        };
        return Some(VulkanRuntime::Missing { detail });
    }
    let address = unsafe { libc::dlsym(library, c"vkEnumerateInstanceVersion".as_ptr()) };
    let query =
        (!address.is_null()).then(|| unsafe { std::mem::transmute::<_, QueryVersion>(address) });
    let report = describe_version(query);
    unsafe { libc::dlclose(library) };
    Some(report)
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn vulkan_runtime() -> Option<VulkanRuntime> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinguishes_a_missing_volume_from_a_measured_volume() {
        let temp = tempfile::tempdir().unwrap();
        let result = volume_space(&temp.path().join("absent"));
        assert!(result.available_bytes.is_none());
        assert!(result.error.is_some());
        assert!(volume_space(temp.path())
            .total_bytes
            .is_some_and(|bytes| bytes > 0));
    }

    #[test]
    fn permission_probe_creates_no_files_and_detects_invalid_output_directory() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("Output"), "file instead of a directory").unwrap();
        let result = directory_access(temp.path());
        assert_eq!(result.blocked, ["Output: not a directory"]);
        assert!(result.errors.is_empty());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[cfg(any(windows, target_os = "linux"))]
    #[test]
    fn rejects_vulkan_1_0_and_accepts_vulkan_1_3() {
        unsafe extern "system" fn version_1_3(version: *mut u32) -> i32 {
            unsafe { *version = (1 << 22) | (3 << 12) };
            0
        }
        assert!(matches!(
            describe_version(None),
            VulkanRuntime::Available {
                meets_minimum: false,
                ..
            }
        ));
        assert!(matches!(
            describe_version(Some(version_1_3)),
            VulkanRuntime::Available {
                meets_minimum: true,
                ..
            }
        ));
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires a host with a Vulkan 1.3 or later system loader"]
    fn verifies_host_vulkan_loader() {
        let result = vulkan_runtime();
        assert!(
            matches!(
                result,
                Some(VulkanRuntime::Available {
                    meets_minimum: true,
                    ..
                })
            ),
            "{result:?}"
        );
    }
}
