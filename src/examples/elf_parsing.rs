use alloc::ffi::CString;
use elf::{
    abi::{DT_NEEDED, DT_RPATH},
    endian::AnyEndian,
    ElfBytes,
};
use rustix::fs::{Mode, OFlags};

use crate::println;

fn some_elf() {
    let path = CString::new("/proc/self/exe").unwrap();
    let fd = rustix::fs::open(&path, OFlags::RDONLY, Mode::RUSR).unwrap();
    // allocate Vec buf size mb
    let mut buf = vec![0; 10_000_000];

    let bytes_read = rustix::io::read(&fd, buf.as_mut_slice()).unwrap();
    println!("Read {:?} bytes\n", bytes_read);
    let data = &buf[..bytes_read];

    let file = ElfBytes::<AnyEndian>::minimal_parse(data).unwrap();
    let common = file.find_common_data().unwrap();

    let rodata = file
        .section_header_by_name(".rodata")
        .unwrap_or_default()
        .unwrap();

    println!("rodata: {:?}", rodata);

    let (dynsyms, strtab) = (common.dynsyms.unwrap(), common.dynsyms_strs.unwrap());
    for s in dynsyms.iter() {
        let x = strtab.get(s.st_name as usize).ok().unwrap_or_default();
        println!("S: {:?}", x);
    }

    let d = common.dynamic.unwrap();

    for d in d.iter() {
        if d.d_tag == DT_NEEDED {
            let x = strtab.get(d.d_ptr() as usize).unwrap_or_default();
            println!("a: {:?}", x);
        }
        if d.d_tag == elf::abi::DT_RUNPATH {
            let x = strtab.get(d.d_ptr() as usize).unwrap_or_default();
            println!("DT_RUNPATH: {:?}", x);
        }
        if d.d_tag == DT_RPATH {
            let x = strtab.get(d.d_ptr() as usize).unwrap_or_default();
            println!("RPATH: {:?}", x);
        }
    }
}
