

use alloc::collections::BTreeMap;
use core::result::Result;
use rustix::fs;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::string::ToString;

pub fn file_exists<P: rustix::path::Arg>(path: P) -> bool {
    rustix::fs::stat(path).is_ok()
}
pub fn load_env_from_file<P: rustix::path::Arg>(envp: &mut super::Envp, path: P) -> Result<(), rustix::io::Errno> {
    let file = rustix::fs::open(path, rustix::fs::OFlags::RDONLY, rustix::fs::Mode::empty())?;
    let mut buffer = Vec::new();
    
    // Read the file contents
    loop {
        let mut chunk = [0u8; 1024];
        match rustix::io::read(&file, &mut chunk) {
            Ok(0) => break, // End of file
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(e) => return Err(e),
        }
    }
    
    // Process each line
    if let Ok(content) = core::str::from_utf8(&buffer) {
        for line in content.lines() {
            let parts = line.splitn(2, '=').collect::<Vec<_>>();
            if parts.len() == 2 {
                envp.insert(parts[0], parts[1]);
            }
        }
    }
    
    Ok(())
}
