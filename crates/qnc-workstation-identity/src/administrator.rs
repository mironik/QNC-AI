//! Whether this process runs with the rights of the computer's administrator (user 2026-11-12:
//! "brisati može jedino pored vlasnika administrator"). Read only, from the system: on Windows the
//! process token is elevated (an administrator account without "Run as administrator" is not);
//! elsewhere the process runs as root. Nothing is guessed: a system it cannot ask says no.

/// True when this process has the administrator's rights.
pub fn is_administrator() -> bool {
    imp::is_administrator()
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    pub(super) fn is_administrator() -> bool {
        // SAFETY: the token of this process, asked once and closed; the answer is a plain struct.
        unsafe {
            let mut token: HANDLE = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
                return false;
            }
            let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
            let mut size = 0u32;
            let asked = GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            );
            CloseHandle(token);
            asked != 0 && elevation.TokenIsElevated != 0
        }
    }
}

#[cfg(unix)]
mod imp {
    pub(super) fn is_administrator() -> bool {
        // SAFETY: geteuid has no arguments and cannot fail.
        unsafe { libc::geteuid() == 0 }
    }
}

#[cfg(not(any(windows, unix)))]
mod imp {
    pub(super) fn is_administrator() -> bool {
        false
    }
}
