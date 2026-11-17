//! The programs one program started, and every program they start, held as one (user 2026-11-17:
//! "ako se gasi tray moraju prestati svi procesi koji potječu od njega jer korisnik nema
//! indikaciju da nešto radi u pozadini"). On Windows it is a job object that kills its processes
//! when its last handle closes: the holder ending, by quitting or by a crash, ends them all. A
//! program started by a program in the job is in it too (it is not let break away).
//!
//! Elsewhere it holds nothing yet: `take` does nothing, `active` is none and `stop` stops nothing
//! (a process group there is a later step).

use std::process::Child;

/// The processes held as one.
pub struct Job {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
}

impl Job {
    /// A new, empty job whose processes end when it is dropped.
    pub fn new() -> Result<Self, String> {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::JobObjects::{CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE};
            // SAFETY: plain Win32 calls on a handle this function owns; the structure is zeroed and sized.
            unsafe {
                let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if handle.is_null() {
                    return Err(format!("job: {}", std::io::Error::last_os_error()));
                }
                let job = Self { handle };
                let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let size = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32;
                if SetInformationJobObject(job.handle, JobObjectExtendedLimitInformation, std::ptr::from_ref(&limits).cast(), size) == 0 {
                    return Err(format!("job limits: {}", std::io::Error::last_os_error()));
                }
                Ok(job)
            }
        }
        #[cfg(not(windows))]
        Ok(Self {})
    }

    /// `child` and every program it starts from now on belong to the job.
    pub fn take(&self, child: &Child) -> Result<(), String> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            // SAFETY: both handles are open for the length of the call.
            if unsafe { windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(self.handle, child.as_raw_handle().cast()) } == 0 {
                return Err(format!("job: {}", std::io::Error::last_os_error()));
            }
        }
        #[cfg(not(windows))]
        let _ = child;
        Ok(())
    }

    /// How many processes of the job run now.
    pub fn active(&self) -> usize {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::JobObjects::{JobObjectBasicAccountingInformation, QueryInformationJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION};
            // SAFETY: the structure is zeroed and sized; the handle is open.
            unsafe {
                let mut info: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = std::mem::zeroed();
                let size = std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32;
                if QueryInformationJobObject(self.handle, JobObjectBasicAccountingInformation, std::ptr::from_mut(&mut info).cast(), size, std::ptr::null_mut()) == 0 {
                    return 0;
                }
                info.ActiveProcesses as usize
            }
        }
        #[cfg(not(windows))]
        0
    }

    /// Ends every process of the job now; the job stays, for the next ones.
    pub fn stop(&self) -> Result<(), String> {
        #[cfg(windows)]
        // SAFETY: the handle is open.
        if unsafe { windows_sys::Win32::System::JobObjects::TerminateJobObject(self.handle, 1) } == 0 {
            return Err(format!("job: {}", std::io::Error::last_os_error()));
        }
        Ok(())
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // The last handle: the processes end with it.
        #[cfg(windows)]
        // SAFETY: the handle is this job's own and closed once.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn sleeper() -> std::process::Child {
        // A program that waits until it is ended.
        std::process::Command::new("cmd").args(["/C", "ping", "-n", "30", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap()
    }

    #[test]
    fn its_processes_and_theirs_end_when_stopped_and_when_it_is_dropped() {
        let job = Job::new().unwrap();
        let mut first = sleeper();
        job.take(&first).unwrap();
        // cmd starts ping: both are in the job.
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(job.active() >= 2, "cmd and the ping it started: {}", job.active());
        job.stop().unwrap();
        assert!(first.wait().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(job.active(), 0);
        let mut second = sleeper();
        job.take(&second).unwrap();
        drop(job);
        assert!(second.wait().is_ok(), "dropping the job ends its processes");
    }
}
