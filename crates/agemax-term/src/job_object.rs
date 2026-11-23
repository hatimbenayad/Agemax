use thiserror::Error;

#[derive(Error, Debug)]
pub enum JobObjectError {
    #[error("Failed to create Job Object: {0}")]
    CreateFailed(String),
    #[error("Failed to configure Job Object: {0}")]
    ConfigureFailed(String),
    #[error("Failed to assign process to Job Object: {0}")]
    AssignFailed(String),
}

#[cfg(windows)]
pub struct JobObject {
    handle: windows::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
unsafe impl Send for JobObject {}

#[cfg(windows)]
unsafe impl Sync for JobObject {}

#[cfg(windows)]
impl JobObject {
    pub fn create() -> Result<Self, JobObjectError> {
        use windows::Win32::System::JobObjects::{
            CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        unsafe {
            let handle = CreateJobObjectW(None, None)
                .map_err(|e| JobObjectError::CreateFailed(e.to_string()))?;

            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let info_ptr = &info as *const _ as *const std::ffi::c_void;
            let info_size = std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32;

            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                info_ptr,
                info_size,
            )
            .map_err(|e| JobObjectError::ConfigureFailed(e.to_string()))?;

            Ok(Self { handle })
        }
    }

    pub fn assign_pid(&self, pid: u32) -> Result<(), JobObjectError> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::JobObjects::AssignProcessToJobObject;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
        };

        unsafe {
            let process_handle = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
                .map_err(|e| JobObjectError::AssignFailed(format!("OpenProcess failed: {}", e)))?;

            let res = AssignProcessToJobObject(self.handle, process_handle);
            let _ = CloseHandle(process_handle);

            res.map_err(|e| {
                JobObjectError::AssignFailed(format!("AssignProcessToJobObject failed: {}", e))
            })
        }
    }

    pub fn raw_handle(&self) -> windows::Win32::Foundation::HANDLE {
        self.handle
    }
}

#[cfg(windows)]
impl Drop for JobObject {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

// Non-Windows stub for compilation / tests
#[cfg(not(windows))]
pub struct JobObject;

#[cfg(not(windows))]
impl JobObject {
    pub fn create() -> Result<Self, JobObjectError> {
        Ok(Self)
    }
    pub fn assign_pid(&self, _pid: u32) -> Result<(), JobObjectError> {
        Ok(())
    }
}
