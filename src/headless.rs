use crate::bench::{self, BenchMessage, TestKind, TestResult};
use crate::drives;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;

use crate::app::DEFAULT_SIZE_MB;

pub fn run(path: Option<String>, size_mb: Option<u64>, json: bool) -> io::Result<()> {
    let size_mb = size_mb.unwrap_or(DEFAULT_SIZE_MB);

    let (display_path, test_path) = match path {
        Some(p) => {
            let pb = PathBuf::from(&p);
            if !pb.is_dir() {
                eprintln!("Error: {p} is not a directory");
                std::process::exit(1);
            }
            (pb.clone(), pb)
        }
        None => {
            let drives = drives::detect_drives();
            if drives.is_empty() {
                eprintln!("Error: no drives detected");
                std::process::exit(1);
            }
            (drives[0].mount.clone(), drives[0].test_path.clone())
        }
    };

    if !json {
        eprintln!(
            "diskrush — {} ({}MB test)",
            display_path.display(),
            size_mb
        );
        eprintln!();
    }

    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel.clone();

    ctrlc_flag(&cancel_clone);

    let tp = test_path.clone();
    std::thread::spawn(move || {
        bench::run_suite(tx, tp, size_mb, cancel_clone, None);
    });

    let mut results: Vec<(TestKind, TestResult)> = Vec::new();

    while let Ok(msg) = rx.recv() {
        match msg {
            BenchMessage::TestStarted { test } => {
                if !json {
                    eprint!("  {} ...", test.label());
                }
            }
            BenchMessage::TestComplete { test, result } => {
                if !json {
                    eprintln!(" {:.1} MB/s ({:.2}s)", result.throughput_mbps, result.duration.as_secs_f64());
                }
                results.push((test, result));
            }
            BenchMessage::SuiteComplete => break,
            BenchMessage::Error(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
            _ => {}
        }
    }

    if json {
        print_json(&display_path, &results);
    } else {
        eprintln!();
        print_table(&results);
    }

    Ok(())
}

fn print_table(results: &[(TestKind, TestResult)]) {
    println!("{:<25} {:>12}", "Test", "Throughput");
    println!("{}", "─".repeat(39));
    for (kind, result) in results {
        println!(
            "{:<25} {:>8.1} MB/s",
            kind.label(),
            result.throughput_mbps
        );
    }
}

fn print_json(path: &Path, results: &[(TestKind, TestResult)]) {
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    map.insert("path", serde_json::Value::String(path.display().to_string()));

    let mut tests = BTreeMap::new();
    for (kind, result) in results {
        let key = match kind {
            TestKind::SeqWrite => "seq_write",
            TestKind::SeqRead => "seq_read",
            TestKind::RandWrite => "rand_write_4k",
            TestKind::RandRead => "rand_read_4k",
        };
        tests.insert(
            key,
            serde_json::json!({
                "throughput_mbps": (result.throughput_mbps * 10.0).round() / 10.0,
                "duration_secs": (result.duration.as_secs_f64() * 100.0).round() / 100.0,
            }),
        );
    }
    map.insert("results", serde_json::to_value(tests).unwrap());

    println!("{}", serde_json::to_string_pretty(&map).unwrap());
}

fn ctrlc_flag(cancel: &Arc<AtomicBool>) {
    let c = cancel.clone();
    let _ = std::thread::spawn(move || {
        // Simple approach: set cancel on SIGINT via crossterm's event system
        // We just let the OS default handle kill the process on double ctrl-c
        use std::sync::atomic::Ordering;
        let _ = ctrlc_wait();
        c.store(true, Ordering::Relaxed);
    });
}

fn ctrlc_wait() -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static GOT_SIGINT: AtomicBool = AtomicBool::new(false);
        unsafe {
            libc::signal(libc::SIGINT, sigint_handler as *const () as libc::sighandler_t);
        }
        while !GOT_SIGINT.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        extern "C" fn sigint_handler(_: libc::c_int) {
            GOT_SIGINT.store(true, Ordering::Relaxed);
        }
    }
    #[cfg(not(unix))]
    {
        // Block forever — the OS default handler will kill the process on Ctrl+C
        loop {
            std::thread::park();
        }
    }
    #[cfg(unix)]
    Ok(())
}
