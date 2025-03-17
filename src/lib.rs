#![no_std]
#![feature(core_intrinsics)]
#![feature(naked_functions)]
#![feature(linkage)]

#[macro_use]
extern crate alloc;

use core::{ffi::c_void, sync::atomic::AtomicPtr};

use alloc::{
    borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, ffi::CString, string::String, vec::Vec,
};
use bstr::ByteSlice;
use http::download_settings;
use rustix::{
    cstr,
    fd::{AsRawFd, IntoRawFd, OwnedFd, RawFd},
    fs::{Mode, OFlags},
    io::fcntl_setfd,
    pipe::{fcntl_setpipe_size, PipeFlags},
    process::WaitOptions,
    runtime::Fork,
    thread::Pid,
};
use rustix_dlmalloc::GlobalDlmalloc;
use settings::RemoteSettings;
use utils::{sleep_nsecs, Argv, NANOSECONDS_PER_SECOND};
mod utils;
use utils::envp::{Envp, EnvpRef};

pub mod dns;
pub mod examples;
mod http;
pub mod settings;

pub mod env;


// #[panic_handler]
// fn panic(_panic: &core::panic::PanicInfo<'_>) -> ! {
//     core::intrinsics::abort()
// }

#[global_allocator]
static DLMALLOC: GlobalDlmalloc = GlobalDlmalloc;

#[used]
#[cfg_attr(
    any(target_os = "linux", target_os = "android"),
    link_section = ".init_array"
)]
static DO_INIT: extern "C" fn(argc: i32, argv: *mut *mut u8, envp: *mut *mut u8) = do_init;

extern "C" {
    #[linkage = "extern_weak"]
    static Py_Version: *const core::ffi::c_void;
}

fn py_version_print() {
    if unsafe { Py_Version.is_null() } {
        return;
    }
    // copilot warning:

    let version = unsafe { *(Py_Version as *const u32) };
    let major = version >> 24;
    let minor = (version >> 16) & 0xff;
    let micro = (version >> 8) & 0xff;
    let releaselevel = version & 0xff;

    print!("Python version: {major}.{minor}.{micro}-{releaselevel}\n");
}

pub struct Background {
    thread: origin::thread::Thread,
}

impl Background {
    pub fn join(self) {
        unsafe { origin::thread::join(self.thread).unwrap() };
    }
}

fn env_loop(child_env: ChildEnv) {
    rustix::thread::set_name(cstr!("new_env_watcher")).unwrap();

    let file_path = CString::new(".new_env").unwrap();

    loop {
        let res = rustix::fs::stat(&file_path);

        if let Ok(stat) = res {
            let fd = rustix::fs::open(&file_path, OFlags::RDONLY, Mode::RUSR).unwrap();
            let mut buf = [0u8; 2024];
            let read_bytes = rustix::io::read(fd, &mut buf).unwrap();
            let s = core::str::from_utf8(&buf[..read_bytes]).unwrap();

            // Copilot: parse str as key=val
            let mut env = BTreeMap::new();
            for line in s.lines() {
                let mut parts = line.split('=');
                let key = parts.next().unwrap();
                let val = parts.next().unwrap();
                env.insert(key.to_owned(), val.to_owned());
            }
            rustix::fs::unlink(&file_path).unwrap();

            CHILD_RESTARTING.store(true, core::sync::atomic::Ordering::SeqCst);

            let pid = CHILD_PID.load(core::sync::atomic::Ordering::SeqCst);
            print!("killing pid: {:?}\n", pid);
            let pid = Pid::from_raw(pid).unwrap();
            let res = rustix::process::kill_process(pid, rustix::process::Signal::Term);
            print!("kill returned {:?}\n", res);

            let mut ce: ChildEnv = child_env.clone();

            for (k, v) in env.iter() {
                ce.env.insert(k, v);
            }
            print!("starting new child thread\n");
            child_thread(ce);
        };

        sleep_nsecs(NANOSECONDS_PER_SECOND / 2);
    }
}

fn remote_env_loop(child_env: ChildEnv) {
    rustix::thread::set_name(cstr!("remote_env_watcher")).unwrap();

    loop {
        let old_gen = RemoteSettings::get_generation();
        let res = download_settings();

        if let Err(err) = res {
            println!("Error downloading settings: {:?}", err);
        }

        let new_gen = RemoteSettings::get_generation();

        if old_gen != new_gen {
            let settings = RemoteSettings::get().unwrap_or_default();
            let mut ce: ChildEnv = child_env.clone();

            for (k, v) in settings.env.iter() {
                ce.env.insert(k, v);
            }

            if let Some(url) = &settings.java_agent_url {
                let res = http::download_java(&url);
                if let Ok(Some(fd)) = res {
                    let raw_fd = fd.into_raw_fd();
                    let java_opts = format!("-javaagent:/proc/self/fd/{}", raw_fd);
                    ce.env.insert(
                        "JAVA_AGENT_FD",
                        format!("/proc/self/fd/{}", raw_fd),
                    );
                    ce.env.insert("JAVA_TOOL_OPTIONS", java_opts);
                    ce.fds_to_drop_in_parent.push(raw_fd);
                }
            }
            let pid = CHILD_PID.load(core::sync::atomic::Ordering::SeqCst);
            if pid != 0 {
                CHILD_RESTARTING.store(true, core::sync::atomic::Ordering::SeqCst);
                let pid = CHILD_PID.load(core::sync::atomic::Ordering::SeqCst);
                println!("killing pid: {:?}\n", pid);
                let pid = Pid::from_raw(pid).unwrap();
                let res = rustix::process::kill_process(pid, rustix::process::Signal::Term);
                println!("kill returned {:?}\n", res);
    
                print!("starting new child thread\n");
                child_thread(ce);
            }
        };

        sleep_nsecs(NANOSECONDS_PER_SECOND * 2);
    }
}

static ARGV: AtomicPtr<*mut u8> = AtomicPtr::new(core::ptr::null_mut());

fn new_env_loop(child_env: &ChildEnv) -> Background {
    let ce = child_env.clone().leak_non_null();

    let thread = unsafe {
        origin::thread::create(
            |_args| {
                let ce = _args[0].unwrap();
                let child_env = ChildEnv::from_non_null(ce);
                env_loop(child_env);
                None
            },
            &[Some(ce)],
            origin::thread::default_stack_size(),
            origin::thread::default_guard_size(),
        )
        .unwrap()
    };

    Background { thread }
}

fn new_remote_env_loop(child_env: &ChildEnv) -> Background {
    let ce = child_env.clone().leak_non_null();

    let thread = unsafe {
        origin::thread::create(
            |_args| {
                let ce = _args[0].unwrap();
                let child_env = ChildEnv::from_non_null(ce);
                remote_env_loop(child_env);
                None
            },
            &[Some(ce)],
            origin::thread::default_stack_size(),
            origin::thread::default_guard_size(),
        )
        .unwrap()
    };

    Background { thread }
}

static CHILD_PID: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(0);
static CHILD_RESTARTING: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);

#[derive(Clone)]
struct ChildEnv {
    env: Envp,
    argv: Argv,
    path: CString,
    fds_to_drop_in_parent: Vec<RawFd>,
}

impl ChildEnv {
    fn leak_non_null(self) -> core::ptr::NonNull<c_void> {
        let some: Box<ChildEnv> = Box::new(self);
        let data = Box::<ChildEnv>::leak(some);
        core::ptr::NonNull::from(data).cast::<c_void>()
    }

    unsafe fn from_non_null(ptr: core::ptr::NonNull<c_void>) -> Self {
        let arg = ptr;
        let child_env = unsafe { Box::<ChildEnv>::from_raw(arg.as_ptr() as *mut ChildEnv) };
        *child_env
    }
}

fn child_thread_core(child_env: &ChildEnv) {
    rustix::thread::set_name(cstr!("child_watcher")).unwrap();

    let path = &child_env.path;

    let envp = child_env.env.as_ptr_vec();
    let argv = child_env.argv.as_ptr_vec();

    print!("path: {:?} argv: {:?}\n", path, argv);

    match unsafe { rustix::runtime::fork().unwrap() } {
        Fork::Child(pid) => {
            let res = unsafe {
                rustix::runtime::execve(
                    path,
                    argv.as_ptr() as *const *const u8,
                    envp.as_ptr() as *const *const u8,
                )
            };
            print!("execve returned {:?}\n", res);
        }
        Fork::Parent(pid) => {
            CHILD_PID.store(
                pid.as_raw_nonzero().get(),
                core::sync::atomic::Ordering::SeqCst,
            );
            CHILD_RESTARTING.store(false, core::sync::atomic::Ordering::SeqCst);

            print!("child pid: {:?}\n", pid);
            let waitopts = WaitOptions::empty();
            let res = rustix::process::waitpid(Some(pid), waitopts);
            let mut child_restarting = CHILD_RESTARTING.load(core::sync::atomic::Ordering::SeqCst);
            let current_child_pid = CHILD_PID.load(core::sync::atomic::Ordering::SeqCst);
            if current_child_pid != pid.as_raw_nonzero().get() {
                print!(
                    "child pid changed from {:?} to {:?}\n",
                    pid, current_child_pid
                );
                child_restarting = true;
            }

            print!(
                "waitpid returned {:?}, child_restarting? {:?}\n",
                res, child_restarting,
            );

            if !child_restarting {
                println!("child naturallly exiting");
                rustix::runtime::exit_group(0);
            }
        }
    }
}

fn child_thread(child_env: ChildEnv) -> Background {
    let ptr = child_env.leak_non_null();

    let thread = unsafe {
        origin::thread::create(
            |_args| {
                let ptr = _args[0].unwrap();
                // let child_env = unsafe { Box::<ChildEnv>::from_raw(arg.as_ptr() as *mut ChildEnv) };
                let child_env = ChildEnv::from_non_null(ptr);

                child_thread_core(&child_env);
                None
            },
            &[Some(ptr)],
            origin::thread::default_stack_size(),
            origin::thread::default_guard_size(),
        )
        .unwrap()
    };

    Background { thread }
}

#[no_mangle]
fn origin_main(_argc: usize, argv: *mut *mut u8, envp: *mut *mut u8) -> i32 {
    let mut env = unsafe { EnvpRef::from_raw(envp).to_envp() };
    env.insert("_GUARD_PRELOAD_DD_HACKATHON", "1");
    let pipe = examples::pipe_handling::some_pipe();
    env.insert(
        "HACKATHON_TELEMETRY_PIPE",
        format!("/proc/self/fd/{}", pipe.writer.as_raw_fd()),
    );
    // env.insert("debian_chroot", "hackathon");

    ARGV.store(argv, core::sync::atomic::Ordering::SeqCst);
    let argv = unsafe { Argv::from_raw(argv) };

    let path = CString::new("/proc/self/exe").unwrap();
    
    let env_file = CString::new("/opt/_auto_dd/.new_env").unwrap();

    if env::file_exists(&env_file) {
        env::load_env_from_file(&mut env, &env_file).unwrap();
    }

    let child_env = ChildEnv { env, argv, path, fds_to_drop_in_parent: vec![] };

    // FOR this POC disable the fancy remote env loop
    // new_env_loop(&child_env);
    // let l = new_remote_env_loop(&child_env);
    child_thread(child_env);

    // l.join();
    drop(pipe);
    rustix::runtime::exit_group(0);
}

pub extern "C" fn do_init(_argc: i32, argv: *mut *mut u8, envp: *mut *mut u8) {
    let mem = unsafe { argv.sub(1) };

    let envp = unsafe { EnvpRef::from_raw(envp) };

    if unsafe { envp.contains_prefix("_GUARD_PRELOAD_DD_HACKATHON=") } {
        return;
    }

    unsafe { origin::program::start(mem as _) };
}
