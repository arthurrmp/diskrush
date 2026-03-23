use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Drive {
    pub name: String,
    pub mount: PathBuf,
    pub test_path: PathBuf,
    pub size: Option<u64>,
}

impl Drive {
    pub fn size_label(&self) -> String {
        match self.size {
            Some(bytes) if bytes >= 1_000_000_000_000 => {
                format!("{:.1} TB", bytes as f64 / 1_000_000_000_000.0)
            }
            Some(bytes) if bytes >= 1_000_000_000 => {
                format!("{:.0} GB", bytes as f64 / 1_000_000_000.0)
            }
            Some(bytes) => format!("{:.0} MB", bytes as f64 / 1_000_000.0),
            None => String::new(),
        }
    }
}

pub fn detect_drives() -> Vec<Drive> {
    let mut drives = Vec::new();
    detect_platform(&mut drives);
    drives
}

#[cfg(target_os = "macos")]
fn detect_platform(drives: &mut Vec<Drive>) {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()));

    // Internal SSD (root filesystem). The root mount is read-only on APFS,
    // so we benchmark from $HOME which lives on the same physical disk.
    if let Some(size) = disk_size("/") {
        drives.push(Drive {
            name: "Macintosh HD".into(),
            mount: PathBuf::from("/"),
            test_path: home,
            size: Some(size),
        });
    }

    // Mounted volumes
    let Ok(entries) = std::fs::read_dir("/Volumes") else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();

        // Root volume appears as a symlink/firmlink in /Volumes — skip it
        if name.contains("Macintosh HD") || name.starts_with('.') {
            continue;
        }

        // Only show volumes backed by a real, writable filesystem
        if !is_writable_real_disk(&path) {
            continue;
        }

        let size = disk_size(&path);
        drives.push(Drive {
            name,
            mount: path.clone(),
            test_path: path,
            size,
        });
    }
}

/// Check filesystem type and mount flags via statfs(2).
/// A volume is benchmarkable if it has a real disk-backed filesystem
/// and is both writable and user-browseable (same flags Finder checks).
#[cfg(target_os = "macos")]
fn is_writable_real_disk(path: &Path) -> bool {
    const MNT_RDONLY: u32 = 0x00000001;
    const MNT_DONTBROWSE: u32 = 0x00100000;

    let Some(c_path) = path.to_str().and_then(|s| std::ffi::CString::new(s).ok()) else {
        return false;
    };
    unsafe {
        let mut stat: libc::statfs = std::mem::zeroed();
        if libc::statfs(c_path.as_ptr(), &mut stat) != 0 {
            return false;
        }

        let fstype = std::ffi::CStr::from_ptr(stat.f_fstypename.as_ptr())
            .to_str()
            .unwrap_or("");
        let real_fs = matches!(fstype, "apfs" | "hfs" | "msdos" | "exfat" | "ntfs");
        let hidden = (stat.f_flags & (MNT_RDONLY | MNT_DONTBROWSE)) != 0;

        real_fs && !hidden
    }
}

#[cfg(target_os = "linux")]
fn detect_platform(drives: &mut Vec<Drive>) {
    let Ok(contents) = std::fs::read_to_string("/proc/mounts") else {
        return;
    };
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()));

    for line in contents.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let (device, mount, fstype) = (parts[0], parts[1], parts[2]);

        if !device.starts_with("/dev/") {
            continue;
        }
        if !matches!(
            fstype,
            "ext4" | "ext3" | "ext2" | "xfs" | "btrfs" | "f2fs" | "vfat" | "exfat" | "ntfs" | "ntfs3"
        ) {
            continue;
        }
        if mount.starts_with("/boot") || mount.starts_with("/snap") {
            continue;
        }

        let path = PathBuf::from(mount);
        let name = if mount == "/" {
            "Root".into()
        } else {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| mount.to_string())
        };
        let test_path = if mount == "/" {
            home.clone()
        } else {
            path.clone()
        };
        let size = disk_size(&path);
        drives.push(Drive {
            name,
            mount: path,
            test_path,
            size,
        });
    }
}

#[cfg(target_os = "windows")]
fn detect_platform(drives: &mut Vec<Drive>) {
    for letter in b'C'..=b'Z' {
        let path = PathBuf::from(format!("{}:\\", letter as char));
        if path.exists() {
            drives.push(Drive {
                name: format!("{}:", letter as char),
                mount: path.clone(),
                test_path: path,
                size: None,
            });
        }
    }
}

#[cfg(unix)]
fn disk_size(path: impl AsRef<Path>) -> Option<u64> {
    let c_path = std::ffi::CString::new(path.as_ref().to_str()?).ok()?;
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            Some(stat.f_blocks as u64 * stat.f_frsize)
        } else {
            None
        }
    }
}

#[cfg(not(unix))]
fn disk_size(_path: impl AsRef<Path>) -> Option<u64> {
    None
}
