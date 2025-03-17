use core::ffi::CStr;

use alloc::{borrow::ToOwned, ffi::CString, vec::Vec};

pub struct EnvpRef<'a> {
    env: Vec<&'a CStr>,
}

impl EnvpRef<'_> {
    pub unsafe fn from_raw(envp: *mut *mut u8) -> Self {
        let mut env = Vec::new();
        let mut i = 0;
        loop {
            let s = unsafe { *envp.offset(i) };
            if s.is_null() {
                break;
            }
            env.push(unsafe { CStr::from_ptr(s as *const core::ffi::c_char) });
            i += 1;
        }

        EnvpRef { env }
    }

    pub unsafe fn to_envp(&self) -> Envp {
        let mut env = Vec::new();
        for s in self.env.iter() {
            env.push(s.to_owned().to_owned());
        }
        Envp { env }
    }

    pub unsafe fn contains_prefix<K: AsRef<str>>(&self, key: K) -> bool {
        for s in self.env.iter() {
            if let Ok(s) = s.to_str() {
                if s.starts_with(key.as_ref()) {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Clone)]
pub struct Envp {
    env: Vec<CString>,
}

impl Envp {
    pub fn new() -> Self {
        Envp { env: Vec::new() }
    }

    pub fn insert<K: AsRef<str>, V: AsRef<str>>(&mut self, key: K, val: V) {
        self.env.retain_mut(|s| {
            if let Ok(s) = s.to_str() {
                s.split('=').next() != Some(key.as_ref())
            } else {
                true
            }
        });

        let s = format!("{}={}", key.as_ref(), val.as_ref());

        self.env.push(CString::new(s).unwrap());
    }

    pub fn get_value<K: AsRef<str>>(&self, key: K) -> Option<alloc::string::String> {
        for s in self.env.iter() {
            if let Ok(s) = s.to_str() {
                if let Some((found_key, value)) = s.split_once('=') {
                    if found_key == key.as_ref() {
                        return Some(value.to_owned());
                    }
                }
            }
        }
        None
    }

    pub fn as_ptr_vec(&self) -> Vec<*const core::ffi::c_char> {
        let mut envp = Vec::new();
        for s in self.env.iter() {
            envp.push(s.as_ptr());
        }

        envp.push(core::ptr::null::<core::ffi::c_char>());

        envp
    }
}
