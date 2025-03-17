use core::ffi::CStr;

use alloc::{borrow::ToOwned, ffi::CString, vec::Vec};

pub mod envp;

pub fn do_print<T: AsRef<str>>(msg: T) {
    let bytes = msg.as_ref().as_bytes();
    let stdout = unsafe { rustix::stdio::stderr() };
    rustix::io::write(stdout, bytes).unwrap();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        let msg = format!($($arg)*);
        $crate::utils::do_print(msg);
    })
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        let msg = format!($($arg)*) + "\n";
        $crate::utils::do_print(msg);
    })
}

pub fn sleep_nsecs(nsecs: u64) {
    let secs = nsecs / NANOSECONDS_PER_SECOND;
    let nsecs = nsecs % NANOSECONDS_PER_SECOND;

    let request = rustix::fs::Timespec {
        tv_sec: secs as i64,
        tv_nsec: nsecs as i64,
    };

    let res = rustix::thread::nanosleep(&request);
}

pub const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000;

#[derive(Clone)]
pub struct Argv {
    args: Vec<CString>,
}

impl Argv {
    pub fn new() -> Self {
        Argv { args: Vec::new() }
    }

    pub unsafe fn from_raw(argv: *mut *mut u8) -> Self {
        let mut args = Vec::new();
        let mut i = 0;
        loop {
            let s = unsafe { *argv.offset(i) };
            if s.is_null() {
                break;
            }
            args.push(unsafe { CStr::from_ptr(s as *mut i8).to_owned() });
            i += 1;
        }

        Argv { args }
    }

    pub fn as_ptr_vec(&self) -> Vec<*mut u8> {
        let mut raw = Vec::new();
        for s in self.args.iter() {
            raw.push(s.as_ptr() as *mut u8);
        }
        raw.push(core::ptr::null_mut());
        raw
    }
}

struct ParamsRef<'a> {
    params: Vec<&'a CStr>,
}

impl ParamsRef<'_> {
    pub unsafe fn from_raw(paramsp: *mut *mut u8) -> Self {
        let mut params = Vec::new();
        let mut i = 0;
        loop {
            let s = unsafe { *paramsp.offset(i) };
            if s.is_null() {
                break;
            }
            params.push(unsafe { CStr::from_ptr(s as *const core::ffi::c_char) });
            i += 1;
        }

        ParamsRef { params }
    }

    pub unsafe fn contains_prefix<K: AsRef<str>>(&self, key: K) -> bool {
        for s in self.params.iter() {
            if let Ok(s) = s.to_str() {
                if s.starts_with(key.as_ref()) {
                    return true;
                }
            }
        }
        false
    }
}
