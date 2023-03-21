use windows::Win32::Foundation::{GetLastError, BOOL};

use crate::ExpectWithTracing;

impl ExpectWithTracing<()> for BOOL {
    fn expectx<S: AsRef<str>>(self, msg: S) -> () {
        self.ok().expectx(msg)
    }
}

pub fn panic_win32_error(msg: &'static str) -> ! {
    log::error!("{:?}", msg);
    unsafe { GetLastError().ok().expect(msg) };
    panic!("{}", msg)
}
