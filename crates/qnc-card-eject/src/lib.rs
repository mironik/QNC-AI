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
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::{
        CM_Get_Parent, CM_Request_Device_EjectW, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
        SetupDiGetDeviceInterfaceDetailW, CR_SUCCESS, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, PNP_VETO_TYPE, SP_DEVICE_INTERFACE_DATA,
        SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA,
    };
    use windows_sys::Win32::System::Ioctl::{
        FSCTL_DISMOUNT_VOLUME, FSCTL_LOCK_VOLUME, GUID_DEVINTERFACE_DISK, IOCTL_STORAGE_EJECT_MEDIA, IOCTL_STORAGE_GET_DEVICE_NUMBER,
        IOCTL_STORAGE_MEDIA_REMOVAL, PREVENT_MEDIA_REMOVAL, STORAGE_DEVICE_NUMBER,
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
        // The disk's number, to find its device for the system's eject below.
        let number = device_number(volume.0);
        control(&volume, IOCTL_STORAGE_EJECT_MEDIA, std::ptr::null(), 0).map_err(|code| EjectError::Failed(format!("not ejected ({code})")))?;
        // The volume let go first: the system refuses to remove a device something holds.
        drop(volume);
        // A card reader often mounts the card again at once after the medium eject (live
        // 2026-11-14): the device is then removed as the system's "Safely remove" does (user
        // 2026-11-14: "aktivirati sistemsko izbacivanje kartice"), so the card stays out.
        match number {
            Some(number) => remove_device(number),
            None => Ok(()),
        }
    }

    /// The storage device number of the open volume `handle`.
    fn device_number(handle: HANDLE) -> Option<STORAGE_DEVICE_NUMBER> {
        let mut number: STORAGE_DEVICE_NUMBER = unsafe { std::mem::zeroed() };
        let mut returned = 0u32;
        // SAFETY: the handle is open; the output buffer is a STORAGE_DEVICE_NUMBER that lives through the call.
        let ok = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_GET_DEVICE_NUMBER,
                std::ptr::null(),
                0,
                (&mut number as *mut STORAGE_DEVICE_NUMBER).cast(),
                std::mem::size_of::<STORAGE_DEVICE_NUMBER>() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        (ok != 0).then_some(number)
    }

    /// The system's eject of the disk device with this number (and, when the disk refuses, of the
    /// device it hangs on, the card reader), as "Safely remove hardware". A device the system
    /// does not let go is the system's reason (veto) as the error.
    fn remove_device(number: STORAGE_DEVICE_NUMBER) -> Result<(), EjectError> {
        let Some(instance) = disk_instance(number) else {
            return Err(EjectError::Failed("the disk's device was not found for the system eject".into()));
        };
        let mut veto = String::new();
        for target in [Some(instance), parent(instance)].into_iter().flatten() {
            let mut kind: PNP_VETO_TYPE = 0;
            let mut name = [0u16; 260];
            // SAFETY: the device instance is valid; the veto name buffer lives through the call.
            let result = unsafe { CM_Request_Device_EjectW(target, &mut kind, name.as_mut_ptr(), name.len() as u32, 0) };
            if result == CR_SUCCESS {
                return Ok(());
            }
            let end = name.iter().position(|&unit| unit == 0).unwrap_or(name.len());
            veto = format!("system eject refused (cr {result}, veto {kind} {})", String::from_utf16_lossy(&name[..end]));
        }
        Err(EjectError::Failed(veto))
    }

    fn parent(instance: u32) -> Option<u32> {
        let mut parent = 0u32;
        // SAFETY: the device instance is valid; `parent` lives through the call.
        (unsafe { CM_Get_Parent(&mut parent, instance, 0) } == CR_SUCCESS).then_some(parent)
    }

    /// The device instance of the present disk whose storage number is `number`.
    fn disk_instance(number: STORAGE_DEVICE_NUMBER) -> Option<u32> {
        // SAFETY: the class GUID is a static; the list is destroyed below.
        let set = unsafe { SetupDiGetClassDevsW(&GUID_DEVINTERFACE_DISK, std::ptr::null(), std::ptr::null_mut(), DIGCF_PRESENT | DIGCF_DEVICEINTERFACE) };
        if set as isize == INVALID_HANDLE_VALUE as isize {
            return None;
        }
        let mut found = None;
        for index in 0.. {
            let mut interface: SP_DEVICE_INTERFACE_DATA = unsafe { std::mem::zeroed() };
            interface.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
            // SAFETY: the set is open; `interface` lives through the call.
            if unsafe { SetupDiEnumDeviceInterfaces(set, std::ptr::null(), &GUID_DEVINTERFACE_DISK, index, &mut interface) } == 0 {
                break;
            }
            // The detail is a header and the path after it: a buffer of u32 for its alignment.
            let mut buffer = vec![0u32; 512];
            let detail = buffer.as_mut_ptr().cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
            // SAFETY: the buffer is large enough for the header; cbSize is the header's size.
            unsafe { (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32 };
            let mut info: SP_DEVINFO_DATA = unsafe { std::mem::zeroed() };
            info.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
            // SAFETY: the buffer holds (buffer.len() * 4) bytes; `info` lives through the call.
            let ok = unsafe { SetupDiGetDeviceInterfaceDetailW(set, &interface, detail, (buffer.len() * 4) as u32, std::ptr::null_mut(), &mut info) };
            if ok == 0 {
                continue;
            }
            // SAFETY: the path is a NUL-terminated string inside the buffer.
            let path = unsafe { std::ptr::addr_of!((*detail).DevicePath) }.cast::<u16>();
            // SAFETY: as above; opened without access, only to ask its number.
            let handle = unsafe { CreateFileW(path, 0, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut()) };
            if handle == INVALID_HANDLE_VALUE {
                continue;
            }
            let disk = Volume(handle);
            if device_number(disk.0).is_some_and(|other| other.DeviceNumber == number.DeviceNumber && other.DeviceType == number.DeviceType) {
                found = Some(info.DevInst);
                break;
            }
        }
        // SAFETY: the set opened above.
        unsafe { SetupDiDestroyDeviceInfoList(set) };
        found
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
