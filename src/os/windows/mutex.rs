use anyhow::{bail, Result};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE},
        Security::SECURITY_ATTRIBUTES,
        System::Threading::CreateMutexW,
    },
};

pub fn create_app_mutex(name: &str, sa: Option<SECURITY_ATTRIBUTES>) -> Result<HANDLE> {
    unsafe {
        let sa = match &sa {
            Some(sa) => Some(sa as *const _),
            None => None,
        };
        let mutex = CreateMutexW(sa, true, &HSTRING::from(name))?;

        if GetLastError() == ERROR_ALREADY_EXISTS {
            bail!("Already running")
        }
        Ok(mutex)
    }
}
