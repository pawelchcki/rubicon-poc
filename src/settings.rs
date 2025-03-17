use core::sync::atomic::{AtomicPtr, AtomicU64};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, string::String, vec::Vec};
use serde::{Deserialize, Serialize};

use crate::println;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct RemoteSettings {
    #[serde(default)]
    generation: u64,
    #[serde(default)]
    pub java_agent_url: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

pub static SETTINGS_GEN: AtomicU64 = AtomicU64::new(0);

static SETTINGS: AtomicPtr<RemoteSettings> = AtomicPtr::new(core::ptr::null_mut());
static OLD_SETTINGS: AtomicPtr<RemoteSettings> = AtomicPtr::new(core::ptr::null_mut());

impl RemoteSettings {
    pub fn get() -> Option<RemoteSettings> {
        unsafe {
            SETTINGS
                .load(core::sync::atomic::Ordering::Relaxed)
                .as_ref().map(|s|s.clone()) //clone ASAP to avoid contention
        }
    }
    
    pub fn get_generation() -> u64 {
        SETTINGS_GEN.load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn store(self) {
        let generation = self.generation;
        let old_generation = SETTINGS_GEN.load(core::sync::atomic::Ordering::Relaxed);

        if generation == old_generation {
            return;
        }
        println!("new settings downloaded, storing: {:?}", self);


        let old = SETTINGS.swap(Box::into_raw(Box::new(self)), core::sync::atomic::Ordering::SeqCst);
        // I don't like just leaking things - lets try ultra optimistic memory management
        let probably_forgotten_by_now = OLD_SETTINGS.swap(old, core::sync::atomic::Ordering::SeqCst);

        if (probably_forgotten_by_now as *const RemoteSettings) != core::ptr::null() {
            let _free = unsafe { Box::from_raw(probably_forgotten_by_now) };
        }
        
        SETTINGS_GEN.store(generation, core::sync::atomic::Ordering::Relaxed);
    }
}
