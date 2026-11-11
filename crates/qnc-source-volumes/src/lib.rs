//! The disks attached to this computer now, with their volume serial number (user
//! 2026-10-16: a USB disk or card the system sees is a source without being added by
//! hand). Read only: it lists, it never mounts, writes or scans.
//!
//! Windows: every drive letter with a file system and its volume serial. Other systems
//! do not expose a volume serial the same way; there the list is empty for now and the
//! host configuration names the sources.

use std::path::PathBuf;

pub const MODULE_ID: &str = "qnc.module.source-volumes";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// An attached disk: its root, its volume serial (8 hex digits) and its label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    pub root: PathBuf,
    pub serial_number: String,
    pub volume_name: Option<String>,
}

impl Volume {
    /// The QNC source URI of the disk: one per volume serial, whatever letter it got.
    pub fn source_uri(&self) -> String {
        format!("qnc://local/source/volume-{}", self.serial_number)
    }
}

/// What kind of disk a volume is, as the system says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskKind {
    /// A card in a reader, a USB stick (the system calls them removable).
    Removable,
    /// A disk built in or a USB disk the system calls fixed.
    Fixed,
    Network,
    Optical,
    Other,
}

/// What tells two disks apart besides their serial: their kind and size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Description {
    pub kind: DiskKind,
    pub size_bytes: Option<u64>,
}

/// The kind and size of the disk at `root` (read only; nothing is mounted).
pub fn describe(root: &std::path::Path) -> Description {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW};
        let wide: Vec<u16> = root.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        // SAFETY: the path is a zero-terminated wide string living through the calls.
        let kind = match unsafe { GetDriveTypeW(wide.as_ptr()) } {
            2 => DiskKind::Removable,
            3 => DiskKind::Fixed,
            4 => DiskKind::Network,
            5 => DiskKind::Optical,
            _ => DiskKind::Other,
        };
        let mut total = 0u64;
        // SAFETY: as above; the output lives through the call.
        let ok = unsafe { GetDiskFreeSpaceExW(wide.as_ptr(), std::ptr::null_mut(), &mut total, std::ptr::null_mut()) };
        Description { kind, size_bytes: (ok != 0 && total > 0).then_some(total) }
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        Description { kind: DiskKind::Other, size_bytes: None }
    }
}

/// The disks attached now.
pub fn mounted() -> Vec<Volume> {
    #[cfg(windows)]
    {
        (b'A'..=b'Z')
            .filter_map(|letter| {
                let root = PathBuf::from(format!("{}:\\", letter as char));
                if !root.is_dir() {
                    return None;
                }
                let (serial_number, volume_name) = identity(&root)?;
                Some(Volume { root, serial_number, volume_name })
            })
            .collect()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn identity(root: &std::path::Path) -> Option<(String, Option<String>)> {
    use std::{os::windows::ffi::OsStrExt, ptr::null_mut};
    use windows_sys::Win32::Storage::FileSystem::GetVolumeInformationW;
    let wide: Vec<u16> = root.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let mut name = vec![0u16; 260];
    let mut serial = 0u32;
    // SAFETY: the buffers live through the call and their lengths are given.
    let ok = unsafe {
        GetVolumeInformationW(wide.as_ptr(), name.as_mut_ptr(), name.len() as u32, &mut serial, null_mut(), null_mut(), null_mut(), 0)
    };
    if ok == 0 {
        return None;
    }
    let end = name.iter().position(|value| *value == 0).unwrap_or(name.len());
    let label = String::from_utf16_lossy(&name[..end]);
    Some((format!("{serial:08x}"), (!label.trim().is_empty()).then_some(label)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disk_is_a_source_by_its_serial() {
        let volume = Volume { root: PathBuf::from("E:\\"), serial_number: "7e80f1fb".into(), volume_name: None };
        assert_eq!(volume.source_uri(), "qnc://local/source/volume-7e80f1fb");
    }

    #[cfg(windows)]
    #[test]
    fn the_system_disk_is_listed_with_its_serial() {
        let volumes = mounted();
        assert!(volumes.iter().any(|volume| volume.root.to_string_lossy().starts_with("C:") && volume.serial_number.len() == 8));
    }

    #[cfg(windows)]
    #[test]
    fn the_system_disk_is_fixed_and_has_a_size() {
        let description = describe(std::path::Path::new("C:\\"));
        assert_eq!(description.kind, DiskKind::Fixed);
        assert!(description.size_bytes.is_some_and(|bytes| bytes > 1 << 30));
    }
}
