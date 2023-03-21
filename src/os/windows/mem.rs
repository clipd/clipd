use std::marker::PhantomData;

use anyhow::Result;
use windows::Win32::{
    Foundation::{CloseHandle, GetLastError, SetLastError, HANDLE, NO_ERROR},
    System::Memory::{GlobalAlloc, GlobalFree, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
};

pub struct HandleGuard<T>(isize, PhantomData<T>);

impl HandleGuard<std::ffi::c_void> {
    pub fn alloc_zero() -> Result<Self> {
        Self::alloc_moveable(0)
    }
}

impl<T> HandleGuard<T> {
    pub fn from_raw(i: isize) -> Self {
        Self(i, PhantomData)
    }

    pub fn alloc_moveable(len: usize) -> Result<Self> {
        let hmem = unsafe {
            SetLastError(NO_ERROR);
            GlobalAlloc(GMEM_MOVEABLE, len * std::mem::size_of::<T>())
        };
        log::trace!("hmem: 0x{:x}", hmem);
        unsafe { GetLastError().ok()? };
        Ok(Self(hmem, PhantomData))
    }

    pub fn handle(&self) -> HANDLE {
        HANDLE(self.0)
    }

    pub fn lock(&self) -> *mut T {
        log::trace!("lock handle 0x{:x}", self.0);
        unsafe { GlobalLock(self.0) as *mut T }
    }

    pub fn unlock(&self) {
        let r = unsafe { GlobalUnlock(self.0) };
        log::trace!("unlock handle 0x{:x} {:?}", self.0, r);
    }
}

impl<T> From<&HandleGuard<T>> for HANDLE {
    fn from(value: &HandleGuard<T>) -> Self {
        value.handle()
    }
}

impl<T> Drop for HandleGuard<T> {
    fn drop(&mut self) {
        self.unlock();
        let r = unsafe { CloseHandle(*(self.0 as *mut HANDLE)) };
        log::trace!("close handle 0x{:x} {:?}", self.0, r);
        let r = unsafe { GlobalFree(self.0) };
        log::trace!("free hmem 0x{:x} {}", self.0, r);
    }
}
