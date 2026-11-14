//! While it runs, the system's AutoPlay is cancelled for the disks the caller names (user
//! 2026-11-14: "kad se ubacuje kartica koju poznaje tray, da se ne otvara File Explorer, nego
//! import wizard"). Windows asks the running programs before it opens anything for a disk put in
//! (an `IQueryCancelAutoPlay` registered in the running object table); the caller's `cancel`
//! answers for the disk (its root and volume serial). No system setting is changed: when the
//! program ends, AutoPlay is as the user set it. It reads and writes nothing itself.

use std::path::{Path, PathBuf};

/// Whether AutoPlay is cancelled for the disk at this root with this volume serial.
pub type Cancel = Box<dyn Fn(&Path, u32) -> bool + Send + Sync + 'static>;

/// The registration; it ends when this is dropped (or the program ends).
pub struct Guard {
    #[cfg(windows)]
    _inner: windows_adapter::Registered,
}

/// Starts answering AutoPlay's question with `cancel`. Another system: an error (it has no
/// AutoPlay to answer yet).
pub fn guard(cancel: Cancel) -> Result<Guard, String> {
    #[cfg(windows)]
    {
        Ok(Guard { _inner: windows_adapter::register(cancel)? })
    }
    #[cfg(not(windows))]
    {
        let _ = cancel;
        Err("cancelling AutoPlay is not made for this system yet".into())
    }
}

/// The root a path AutoPlay names (`D:\`).
fn root_of(path: &str) -> PathBuf {
    PathBuf::from(path)
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn the_answer_is_registered_and_let_go() {
        let guard = super::guard(Box::new(|_, _| true)).expect("registered");
        drop(guard);
        // Registered again once the first is gone.
        drop(super::guard(Box::new(|_, _| false)).expect("registered again"));
    }
}

#[cfg(windows)]
mod windows_adapter {
    use super::Cancel;
    use std::sync::mpsc::channel;

    use windows::core::{implement, Result, HRESULT, PCWSTR};
    use windows::Win32::Foundation::S_FALSE;
    use windows::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, CreateClassMoniker, GetRunningObjectTable, COINIT_APARTMENTTHREADED, ROTFLAGS_REGISTRATIONKEEPSALIVE,
    };
    use windows::Win32::UI::Shell::{IQueryCancelAutoPlay, IQueryCancelAutoPlay_Impl};

    /// CLSID_QueryCancelAutoPlay (shobjidl): the class moniker AutoPlay looks for in the table.
    #[allow(non_upper_case_globals)]
    const CLSID_QueryCancelAutoPlay: windows::core::GUID = windows::core::GUID::from_u128(0x331f1768_05a9_4ddd_b86e_dae34ddc998a);
    use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, PostThreadMessageW, TranslateMessage, MSG, WM_QUIT};

    #[implement(IQueryCancelAutoPlay)]
    struct Answer {
        cancel: Cancel,
    }

    impl IQueryCancelAutoPlay_Impl for Answer_Impl {
        fn AllowAutoPlay(&self, path: &PCWSTR, _content: u32, _label: &PCWSTR, serial: u32) -> Result<()> {
            // SAFETY: the system passes a NUL-terminated path for the call's length.
            let path = unsafe { path.to_string() }.unwrap_or_default();
            if (self.cancel)(&super::root_of(&path), serial) {
                // S_FALSE: AutoPlay does not open anything for this disk.
                return Err(HRESULT(S_FALSE.0).into());
            }
            Ok(())
        }
    }

    /// The thread that holds the registration; it ends when this is dropped.
    pub(super) struct Registered {
        thread: u32,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    impl Drop for Registered {
        fn drop(&mut self) {
            // SAFETY: posts to the guard's own thread, which ends its message loop.
            unsafe {
                let _ = PostThreadMessageW(self.thread, WM_QUIT, Default::default(), Default::default());
            }
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    pub(super) fn register(cancel: Cancel) -> std::result::Result<Registered, String> {
        let (made, started) = channel::<std::result::Result<u32, String>>();
        let handle = std::thread::spawn(move || {
            // SAFETY: COM on this thread, a single-threaded apartment with its message loop below.
            if let Err(error) = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok() {
                let _ = made.send(Err(format!("COM: {error}")));
                return;
            }
            let registered = (|| -> Result<(windows::Win32::System::Com::IRunningObjectTable, u32)> {
                let answer: IQueryCancelAutoPlay = Answer { cancel }.into();
                // SAFETY: plain COM calls with live arguments.
                let table = unsafe { GetRunningObjectTable(0)? };
                let moniker = unsafe { CreateClassMoniker(&CLSID_QueryCancelAutoPlay)? };
                let cookie = unsafe { table.Register(ROTFLAGS_REGISTRATIONKEEPSALIVE, &answer, &moniker)? };
                Ok((table, cookie))
            })();
            let (table, cookie) = match registered {
                Ok(registered) => registered,
                Err(error) => {
                    let _ = made.send(Err(format!("AutoPlay not registered: {error}")));
                    // SAFETY: pairs the CoInitializeEx above.
                    unsafe { CoUninitialize() };
                    return;
                }
            };
            // SAFETY: the id of this thread.
            let _ = made.send(Ok(unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }));
            let mut message = MSG::default();
            // SAFETY: the message loop of this thread; the system's calls come through it.
            unsafe {
                while GetMessageW(&mut message, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                let _ = table.Revoke(cookie);
                drop(table);
                CoUninitialize();
            }
        });
        let thread = started.recv().map_err(|_| "the AutoPlay thread ended".to_string())??;
        Ok(Registered { thread, handle: Some(handle) })
    }
}
