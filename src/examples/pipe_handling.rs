use core::ffi::c_void;

use alloc::{boxed::Box, string::String};
use bstr::ByteSlice;
use rustix::{cstr, fd::OwnedFd, io::fcntl_setfd, pipe::{fcntl_setpipe_size, PipeFlags}};

use crate::{println, utils::{sleep_nsecs, NANOSECONDS_PER_SECOND}};


pub struct PipeWriterMemo {
    pub writer: OwnedFd,
}

pub fn some_pipe_thead_inner(reader: OwnedFd) {
    rustix::thread::set_name(cstr!("pipe_watcher")).unwrap();

    let mut buf = vec![0u8; MAX_PIPE_SIZE];
    let delim = bstr::BStr::new(b"\0\0");
    loop {
        let bytes_read = rustix::io::read(&reader, &mut buf).unwrap();
        let b = bstr::BStr::new(&buf[..bytes_read]);
        for msg in b.split_str(delim) {
            // println!("\t\tsize: {}", msg.len());
            let msg = String::from_utf8_lossy(msg).into_owned();
            // println!("Read: {:?}\n", msg.len());
        }

        println!("Data received {:}", bytes_read);
        sleep_nsecs(NANOSECONDS_PER_SECOND / 2);
    }
}
// Max size assumes 4096 page size
const MAX_PIPE_SIZE: usize = 1_048_576;

// the size is a multiple of a default page size 4096 rounded up
const PIPE_SIZE: usize = MAX_PIPE_SIZE;

pub fn some_pipe() -> PipeWriterMemo {
    let (reader, writer) = rustix::pipe::pipe_with(PipeFlags::DIRECT).unwrap();
    // unset closex on reader so it stays only on main process
    fcntl_setfd(&reader, rustix::io::FdFlags::CLOEXEC).unwrap();
    // fcntl_setfl(&writer, OFlags::WRONLY).unwrap();
    // fcntl_setfl(&writer, OFlags::NONBLOCK).unwrap();

    fcntl_setpipe_size(&writer, PIPE_SIZE).unwrap();

    let pipe = PipeWriterMemo { writer };

    let ll = Box::new(reader);
    let data = Box::<OwnedFd>::leak(ll);

    let data = core::ptr::NonNull::from(data).cast::<c_void>();
    let thread = unsafe {
        origin::thread::create(
            |_args| {
                let reader = _args[0].unwrap();
                let reader = unsafe { Box::<OwnedFd>::from_raw(reader.as_ptr() as *mut OwnedFd) };
                some_pipe_thead_inner(*reader);
                None
            },
            &[Some(data)],
            origin::thread::default_stack_size(),
            origin::thread::default_guard_size(),
        )
        .unwrap()
    };

    pipe
}
