pub mod engine;
pub mod platform;

use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TestKind {
    SeqWrite,
    SeqRead,
    RandWrite,
    RandRead,
}

impl TestKind {
    pub fn label(&self) -> &'static str {
        match self {
            TestKind::SeqWrite => "Sequential Write",
            TestKind::SeqRead => "Sequential Read",
            TestKind::RandWrite => "Random Write 4K",
            TestKind::RandRead => "Random Read 4K",
        }
    }

    pub fn is_sequential(&self) -> bool {
        matches!(self, TestKind::SeqWrite | TestKind::SeqRead)
    }

    pub fn is_write(&self) -> bool {
        matches!(self, TestKind::SeqWrite | TestKind::RandWrite)
    }
}

impl fmt::Display for TestKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestResult {
    pub throughput_mbps: f64,
    pub duration: Duration,
}

#[allow(dead_code)]
pub enum BenchMessage {
    TestStarted { test: TestKind },
    Progress { test: TestKind, bytes_done: u64, bytes_total: u64, elapsed: Duration },
    TestComplete { test: TestKind, result: TestResult },
    SuiteComplete,
    Error(String),
}

pub fn run_suite(
    tx: Sender<BenchMessage>,
    path: PathBuf,
    size_mb: u64,
    cancel: Arc<AtomicBool>,
    enabled: Option<Vec<TestKind>>,
) {
    let total_bytes = size_mb * 1024 * 1024;
    let random_bytes: u64 = (size_mb / 4).max(64) * 1024 * 1024; // 1/4 of total, min 64MB

    let temp_file = match tempfile::Builder::new()
        .prefix("ssd-bench-")
        .suffix(".tmp")
        .tempfile_in(&path)
    {
        Ok(f) => f,
        Err(e) => {
            let _ = tx.send(BenchMessage::Error(format!("Failed to create temp file: {e}")));
            return;
        }
    };
    let file_path = temp_file.path().to_path_buf();

    let all_tests: &[(TestKind, u64)] = &[
        (TestKind::SeqWrite, total_bytes),
        (TestKind::SeqRead, total_bytes),
        (TestKind::RandWrite, random_bytes),
        (TestKind::RandRead, random_bytes),
    ];

    // If SeqRead is enabled but SeqWrite is not, we still need to write the file first
    let needs_write_file = enabled.as_ref().is_none_or(|e| {
        e.contains(&TestKind::SeqRead) || e.contains(&TestKind::RandWrite) || e.contains(&TestKind::RandRead)
    });
    let skip_seq_write = enabled.as_ref().is_some_and(|e| !e.contains(&TestKind::SeqWrite));

    // Pre-create file if we need it for reads but aren't doing seq write
    if needs_write_file && skip_seq_write {
        // Use a dummy channel to avoid sending phantom progress messages to the UI
        let (dummy_tx, _) = std::sync::mpsc::channel();
        let _ = engine::sequential_write(&file_path, total_bytes, &dummy_tx, TestKind::SeqWrite, &cancel);
        if cancel.load(Ordering::Relaxed) {
            let _ = tx.send(BenchMessage::SuiteComplete);
            drop(temp_file);
            return;
        }
    }

    let tests: Vec<(TestKind, u64)> = all_tests
        .iter()
        .filter(|(kind, _)| enabled.as_ref().is_none_or(|e| e.contains(kind)))
        .copied()
        .collect();

    for (test_kind, test_bytes) in &tests {
        let test_kind = *test_kind;
        let test_bytes = *test_bytes;
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        let _ = tx.send(BenchMessage::TestStarted { test: test_kind });

        let result = match test_kind {
            TestKind::SeqWrite => {
                engine::sequential_write(&file_path, test_bytes, &tx, test_kind, &cancel)
            }
            TestKind::SeqRead => {
                engine::sequential_read(&file_path, test_bytes, &tx, test_kind, &cancel)
            }
            TestKind::RandWrite => {
                engine::random_write(&file_path, test_bytes, &tx, test_kind, &cancel)
            }
            TestKind::RandRead => {
                engine::random_read(&file_path, test_bytes, &tx, test_kind, &cancel)
            }
        };

        match result {
            Ok(test_result) => {
                let _ = tx.send(BenchMessage::TestComplete {
                    test: test_kind,
                    result: test_result,
                });
            }
            Err(e) => {
                if !cancel.load(Ordering::Relaxed) {
                    let _ = tx.send(BenchMessage::Error(format!("{test_kind}: {e}")));
                }
                break;
            }
        }
    }

    let _ = tx.send(BenchMessage::SuiteComplete);
    drop(temp_file);
}
