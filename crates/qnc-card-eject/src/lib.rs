//! A card taken out safely (user 2026-11-11: "treba dodati sigurno izbacivanje kartice"): the disk
//! is locked (nothing may have it open), unmounted, its removal allowed and its medium ejected, as
//! the system's own "Eject". A disk something still has open (the player, an import) is refused,
//! never ejected by force. It writes no file and changes nothing on the card. An adapter per
//! system: Windows (the volume's device controls); another system says it has no eject yet.

use std::path::Path;

/// Why a card was not ejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EjectError {
    /// Something still has the disk open: close it and try again.
    InUse,
    /// The disk could not be ejected: the system's reason.
    Failed(String),
}

impl std::fmt::Display for EjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InUse => write!(f, "the disk is in use"),
            Self::Failed(reason) => write!(f, "{reason}"),
        }
    }
}

/// Ejects the disk whose root is `root` (a drive root such as `D:\`).
pub fn eject(root: &Path) -> Result<(), EjectError> {
    #[cfg(windows)]
    {
        windows::eject(root)
    }
    #[cfg(not(windows))]
    {
        let _ = root;
        Err(EjectError::Failed("ejecting a card is not made for this system yet".into()))
    }
}

#[cfg(windows)]
mod windows {
    use super::EjectError;
    use std::path::Path;
    use std::time::Duration;

    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, IOCTL_STORAGE_EJECT_MEDIA, IOCTL_STORAGE_MEDIA_REMOVAL, PREVENT_MEDIA_REMOVAL,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;

    /// How many times the lock is tried (something may let go of the disk a moment later).
    const LOCK_TRIES: u32 = 10;

    struct Volume(HANDLE);

    impl Drop for Volume {
        fn drop(&mut self) {
            // SAFETY: the handle this value opened.
            unsafe { CloseHandle(self.0) };
        }
    }

    fn control(volume: &Volume, code: u32, input: *const core::ffi::c_void, size: u32) -> Result<(), u32> {
        let mut returned = 0u32;
        // SAFETY: the volume handle is open; the input buffer lives through the call.
        let ok = unsafe { DeviceIoControl(volume.0, code, input, size, std::ptr::null_mut(), 0, &mut returned, std::ptr::null_mut()) };
        if ok == 0 {
            // SAFETY: reads the calling thread's last error.
            Err(unsafe { GetLastError() })
        } else {
            Ok(())
        }
    }

    pub(super) fn eject(root: &Path) -> Result<(), EjectError> {
        let letter = root
            .to_str()
            .and_then(|text| text.chars().next())
            .filter(|letter| letter.is_ascii_alphabetic() && root.to_str().is_some_and(|text| text.len() <= 3))
            .ok_or_else(|| EjectError::Failed(format!("{} is no drive root", root.display())))?;
        let device: Vec<u16> = format!("\\\\.\\{letter}:").encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: the device name lives through the call; the handle is closed by `Volume`.
        let handle = unsafe {
            CreateFileW(device.as_ptr(), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut())
        };
        if handle == INVALID_HANDLE_VALUE {
            // SAFETY: as above.
            return Err(EjectError::Failed(format!("the disk was not opened ({})", unsafe { GetLastError() })));
        }
        let volume = Volume(handle);
        let mut locked = false;
        for _ in 0..LOCK_TRIES {
            if control(&volume, FSCTL_LOCK_VOLUME, std::ptr::null(), 0).is_ok() {
                locked = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !locked {
            return Err(EjectError::InUse);
        }
        control(&volume, FSCTL_DISMOUNT_VOLUME, std::ptr::null(), 0).map_err(|code| EjectError::Failed(format!("not unmounted ({code})")))?;
        let allow = PREVENT_MEDIA_REMOVAL { PreventMediaRemoval: 0 };
        control(&volume, IOCTL_STORAGE_MEDIA_REMOVAL, (&allow as *const PREVENT_MEDIA_REMOVAL).cast(), std::mem::size_of::<PREVENT_MEDIA_REMOVAL>() as u32)
            .map_err(|code| EjectError::Failed(format!("removal not allowed ({code})")))?;
        control(&volume, IOCTL_STORAGE_EJECT_MEDIA, std::ptr::null(), 0).map_err(|code| EjectError::Failed(format!("not ejected ({code})")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_drive_root_is_ejected() {
        assert!(matches!(eject(Path::new("C:\\Users")), Err(EjectError::Failed(_))));
        assert!(matches!(eject(Path::new("no-such-place")), Err(EjectError::Failed(_))));
    }
}
