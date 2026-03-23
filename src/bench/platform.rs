use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

/// Open a file for writing with cache bypass, creating/truncating it.
#[cfg(target_os = "macos")]
pub fn open_direct_write_new(path: &Path) -> io::Result<File> {
    use std::os::unix::io::AsRawFd;
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    set_nocache(file.as_raw_fd())?;
    Ok(file)
}

/// Open an existing file for writing with cache bypass (no truncate).
#[cfg(target_os = "macos")]
pub fn open_direct_write_existing(path: &Path) -> io::Result<File> {
    use std::os::unix::io::AsRawFd;
    let file = OpenOptions::new().write(true).open(path)?;
    set_nocache(file.as_raw_fd())?;
    Ok(file)
}

/// Open a file for reading with cache bypass.
#[cfg(target_os = "macos")]
pub fn open_direct_read(path: &Path) -> io::Result<File> {
    use std::os::unix::io::AsRawFd;
    let file = OpenOptions::new().read(true).open(path)?;
    set_nocache(file.as_raw_fd())?;
    Ok(file)
}

#[cfg(target_os = "macos")]
fn set_nocache(fd: std::os::unix::io::RawFd) -> io::Result<()> {
    let ret = unsafe { libc::fcntl(fd, libc::F_NOCACHE, 1) };
    if ret == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub fn open_direct_write_new(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(libc::O_DIRECT)
        .open(path)
}

#[cfg(target_os = "linux")]
pub fn open_direct_write_existing(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_DIRECT)
        .open(path)
}

#[cfg(target_os = "linux")]
pub fn open_direct_read(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECT)
        .open(path)
}

#[cfg(target_os = "windows")]
pub fn open_direct_write_new(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(0x20000000) // FILE_FLAG_NO_BUFFERING
        .open(path)
}

#[cfg(target_os = "windows")]
pub fn open_direct_write_existing(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .custom_flags(0x20000000)
        .open(path)
}

#[cfg(target_os = "windows")]
pub fn open_direct_read(path: &Path) -> io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(0x20000000)
        .open(path)
}

/// 4096-byte aligned buffer for direct I/O.
pub struct AlignedBuffer {
    ptr: *mut u8,
    layout: std::alloc::Layout,
    len: usize,
}

unsafe impl Send for AlignedBuffer {}

impl AlignedBuffer {
    pub fn new(size: usize) -> Self {
        let layout = std::alloc::Layout::from_size_align(size, 4096).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        // Non-zero pattern to prevent SSD zero-compression cheating
        unsafe { std::ptr::write_bytes(ptr, 0xAA, size) };
        Self {
            ptr,
            layout,
            len: size,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe { std::alloc::dealloc(self.ptr, self.layout) };
    }
}
