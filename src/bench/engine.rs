use super::platform::{self, AlignedBuffer};
use super::{BenchMessage, TestKind, TestResult};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

const SEQ_BLOCK_SIZE: usize = 1024 * 1024; // 1 MB
const RAND_BLOCK_SIZE: usize = 4096; // 4 KB
const PROGRESS_INTERVAL: usize = 256; // Send progress every N random ops

fn safe_throughput(bytes: u64, elapsed: Duration) -> f64 {
    let secs = elapsed.as_secs_f64();
    if secs > 0.0 {
        bytes as f64 / secs / (1024.0 * 1024.0)
    } else {
        0.0
    }
}

pub fn sequential_write(
    path: &Path,
    total_bytes: u64,
    tx: &Sender<BenchMessage>,
    test: TestKind,
    cancel: &AtomicBool,
) -> std::io::Result<TestResult> {
    let mut file = platform::open_direct_write_new(path)?;
    let buf = AlignedBuffer::new(SEQ_BLOCK_SIZE);
    let mut written: u64 = 0;
    let start = Instant::now();

    while written < total_bytes {
        if cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "cancelled",
            ));
        }
        let to_write = SEQ_BLOCK_SIZE.min((total_bytes - written) as usize);
        file.write_all(&buf.as_slice()[..to_write])?;
        written += to_write as u64;
        let _ = tx.send(BenchMessage::Progress {
            test,
            bytes_done: written,
            bytes_total: total_bytes,
            elapsed: start.elapsed(),
        });
    }

    file.sync_all()?;
    let elapsed = start.elapsed();
    let throughput = safe_throughput(written, elapsed);

    Ok(TestResult {
        throughput_mbps: throughput,
        duration: elapsed,
    })
}

pub fn sequential_read(
    path: &Path,
    total_bytes: u64,
    tx: &Sender<BenchMessage>,
    test: TestKind,
    cancel: &AtomicBool,
) -> std::io::Result<TestResult> {
    let mut file = platform::open_direct_read(path)?;
    let mut buf = AlignedBuffer::new(SEQ_BLOCK_SIZE);
    let mut read_total: u64 = 0;
    let start = Instant::now();

    while read_total < total_bytes {
        if cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "cancelled",
            ));
        }
        let to_read = SEQ_BLOCK_SIZE.min((total_bytes - read_total) as usize);
        let n = file.read(&mut buf.as_mut_slice()[..to_read])?;
        if n == 0 {
            break;
        }
        read_total += n as u64;
        let _ = tx.send(BenchMessage::Progress {
            test,
            bytes_done: read_total,
            bytes_total: total_bytes,
            elapsed: start.elapsed(),
        });
    }

    let elapsed = start.elapsed();
    let throughput = safe_throughput(read_total, elapsed);

    Ok(TestResult {
        throughput_mbps: throughput,
        duration: elapsed,
    })
}

pub fn random_write(
    path: &Path,
    total_bytes: u64,
    tx: &Sender<BenchMessage>,
    test: TestKind,
    cancel: &AtomicBool,
) -> std::io::Result<TestResult> {
    let file_len = std::fs::metadata(path)?.len();
    let num_offsets = (file_len / RAND_BLOCK_SIZE as u64) as usize;

    use rand::seq::SliceRandom;
    let mut offsets: Vec<u64> = (0..num_offsets)
        .map(|i| i as u64 * RAND_BLOCK_SIZE as u64)
        .collect();
    offsets.shuffle(&mut rand::rng());

    let total_ops = (total_bytes / RAND_BLOCK_SIZE as u64) as usize;
    let ops_to_do = total_ops.min(offsets.len());
    let total_test_bytes = ops_to_do as u64 * RAND_BLOCK_SIZE as u64;

    let mut file = platform::open_direct_write_existing(path)?;
    let buf = AlignedBuffer::new(RAND_BLOCK_SIZE);
    let start = Instant::now();

    for (i, &offset) in offsets.iter().take(ops_to_do).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "cancelled",
            ));
        }
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(buf.as_slice())?;

        if i % PROGRESS_INTERVAL == 0 {
            let _ = tx.send(BenchMessage::Progress {
                test,
                bytes_done: (i as u64 + 1) * RAND_BLOCK_SIZE as u64,
                bytes_total: total_test_bytes,
                elapsed: start.elapsed(),
            });
        }
    }

    file.sync_all()?;
    let elapsed = start.elapsed();
    let throughput = safe_throughput(ops_to_do as u64 * RAND_BLOCK_SIZE as u64, elapsed);

    Ok(TestResult {
        throughput_mbps: throughput,
        duration: elapsed,
    })
}

pub fn random_read(
    path: &Path,
    total_bytes: u64,
    tx: &Sender<BenchMessage>,
    test: TestKind,
    cancel: &AtomicBool,
) -> std::io::Result<TestResult> {
    let file_len = std::fs::metadata(path)?.len();
    let num_offsets = (file_len / RAND_BLOCK_SIZE as u64) as usize;

    use rand::seq::SliceRandom;
    let mut offsets: Vec<u64> = (0..num_offsets)
        .map(|i| i as u64 * RAND_BLOCK_SIZE as u64)
        .collect();
    offsets.shuffle(&mut rand::rng());

    let total_ops = (total_bytes / RAND_BLOCK_SIZE as u64) as usize;
    let ops_to_do = total_ops.min(offsets.len());
    let total_test_bytes = ops_to_do as u64 * RAND_BLOCK_SIZE as u64;

    let mut file = platform::open_direct_read(path)?;
    let mut buf = AlignedBuffer::new(RAND_BLOCK_SIZE);
    let start = Instant::now();

    for (i, &offset) in offsets.iter().take(ops_to_do).enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "cancelled",
            ));
        }
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(buf.as_mut_slice())?;

        if i % PROGRESS_INTERVAL == 0 {
            let _ = tx.send(BenchMessage::Progress {
                test,
                bytes_done: (i as u64 + 1) * RAND_BLOCK_SIZE as u64,
                bytes_total: total_test_bytes,
                elapsed: start.elapsed(),
            });
        }
    }

    let elapsed = start.elapsed();
    let throughput = safe_throughput(ops_to_do as u64 * RAND_BLOCK_SIZE as u64, elapsed);

    Ok(TestResult {
        throughput_mbps: throughput,
        duration: elapsed,
    })
}
