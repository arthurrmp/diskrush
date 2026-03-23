use crate::bench::{self, BenchMessage, TestKind, TestResult};
use crate::drives::{self, Drive};
use crate::history::{self, HistoryEntry};
use crossterm::event::KeyCode;
use std::path::PathBuf;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;

pub const DEFAULT_SIZE_MB: u64 = 1024;

#[derive(Clone, Copy, PartialEq)]
pub enum View {
    Benchmark,
    History,
    Settings,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SettingsField {
    TestSize,
    Fullscreen,
    SeqWrite,
    SeqRead,
    RandWrite,
    RandRead,
}

impl SettingsField {
    pub const ALL: [SettingsField; 6] = [
        SettingsField::TestSize,
        SettingsField::Fullscreen,
        SettingsField::SeqWrite,
        SettingsField::SeqRead,
        SettingsField::RandWrite,
        SettingsField::RandRead,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&f| f == self).unwrap()
    }
}

pub struct Settings {
    pub test_size_mb: u64,
    pub fullscreen: bool,
    pub seq_write: bool,
    pub seq_read: bool,
    pub rand_write: bool,
    pub rand_read: bool,
    pub focused: SettingsField,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            test_size_mb: DEFAULT_SIZE_MB,
            fullscreen: false,
            seq_write: true,
            seq_read: true,
            rand_write: true,
            rand_read: true,
            focused: SettingsField::TestSize,
        }
    }
}

impl Settings {
    pub fn enabled_tests(&self) -> Vec<TestKind> {
        let mut tests = Vec::new();
        if self.seq_write {
            tests.push(TestKind::SeqWrite);
        }
        if self.seq_read {
            tests.push(TestKind::SeqRead);
        }
        if self.rand_write {
            tests.push(TestKind::RandWrite);
        }
        if self.rand_read {
            tests.push(TestKind::RandRead);
        }
        tests
    }
}

pub enum AppState {
    SelectDrive {
        drives: Vec<Drive>,
        selected: usize,
    },
    Running {
        current_test: TestKind,
        progress: f64,
        live_mbps: f64,
        prev_bytes: u64,
        prev_elapsed: Duration,
        completed: Vec<(TestKind, TestResult)>,
    },
    Complete {
        results: Vec<(TestKind, TestResult)>,
    },
}

pub struct App {
    pub state: AppState,
    pub view: View,
    pub drive_name: String,
    pub display_path: PathBuf,
    pub test_path: PathBuf,
    pub settings: Settings,
    pub spinner_tick: usize,
    pub should_quit: bool,
    pub history: Vec<HistoryEntry>,
    pub history_idx: usize,
    rx: Option<Receiver<BenchMessage>>,
    cancel: Option<Arc<AtomicBool>>,
    _worker: Option<JoinHandle<()>>,
}

const SIZE_OPTIONS: &[u64] = &[64, 128, 256, 512, 1024, 2048, 4096];

fn next_size(current: u64) -> u64 {
    let idx = SIZE_OPTIONS.iter().position(|&s| s == current).unwrap_or(2);
    SIZE_OPTIONS[(idx + 1) % SIZE_OPTIONS.len()]
}

impl App {
    pub fn new() -> Self {
        let drives = drives::detect_drives();
        let history = history::load();
        let mut settings = Settings::default();
        let (saved_size, saved_fullscreen) = history::load_settings();
        if let Some(size) = saved_size {
            settings.test_size_mb = size;
        }
        if let Some(fs) = saved_fullscreen {
            settings.fullscreen = fs;
        }
        Self {
            state: AppState::SelectDrive {
                drives,
                selected: 0,
            },
            view: View::Benchmark,
            drive_name: String::new(),
            display_path: PathBuf::new(),
            test_path: PathBuf::new(),
            settings,
            spinner_tick: 0,
            should_quit: false,
            history,
            history_idx: 0,
            rx: None,
            cancel: None,
            _worker: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        // ←→ switches tabs
        let dir: i8 = match key {
            KeyCode::Right => 1,
            KeyCode::Left => -1,
            _ => 0,
        };
        if dir != 0 {
            if let Some(next) = self.next_tab(dir) {
                self.view = next;
                return;
            }
        }

        match self.view {
            View::Benchmark => self.handle_benchmark_key(key),
            View::History => self.handle_history_key(key),
            View::Settings => self.handle_settings_key(key),
        }
    }

    fn next_tab(&self, dir: i8) -> Option<View> {
        let mut tabs = vec![View::Benchmark];
        if !self.history.is_empty() {
            tabs.push(View::History);
        }
        tabs.push(View::Settings);
        let pos = tabs.iter().position(|&v| v == self.view)?;
        let next = pos as i8 + dir;
        tabs.get(next as usize).copied()
    }

    fn handle_benchmark_key(&mut self, key: KeyCode) {
        match &mut self.state {
            AppState::SelectDrive { drives, selected } => match key {
                KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected + 1 < drives.len() {
                        *selected += 1;
                    }
                }
                KeyCode::Enter => {
                    if drives.is_empty() {
                        return;
                    }
                    let drive = &drives[*selected];
                    self.drive_name = drive.name.clone();
                    self.display_path = drive.mount.clone();
                    self.test_path = drive.test_path.clone();
                    self.start_benchmark();
                }
                _ => {}
            },
            AppState::Running { .. } => match key {
                KeyCode::Char('q') => {
                    self.cancel_benchmark();
                    self.should_quit = true;
                }
                KeyCode::Esc => {
                    self.cancel_benchmark();
                    self.go_back_to_drives();
                }
                _ => {}
            },
            AppState::Complete { .. } => match key {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('r') => self.start_benchmark(),
                KeyCode::Esc => self.go_back_to_drives(),
                _ => {}
            },
        }
    }

    fn handle_history_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => self.view = View::Benchmark,
            KeyCode::Up | KeyCode::Char('k') => {
                if self.history_idx > 0 {
                    self.history_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.history_idx + 1 < self.history.len() {
                    self.history_idx += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_settings_key(&mut self, key: KeyCode) {
        let s = &mut self.settings;
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => self.view = View::Benchmark,
            KeyCode::Up | KeyCode::Char('k') => {
                let idx = s.focused.index();
                if idx > 0 {
                    s.focused = SettingsField::ALL[idx - 1];
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let idx = s.focused.index();
                if idx + 1 < SettingsField::ALL.len() {
                    s.focused = SettingsField::ALL[idx + 1];
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => match s.focused {
                SettingsField::TestSize => {
                    s.test_size_mb = next_size(s.test_size_mb);
                    history::save_test_size(s.test_size_mb);
                }
                SettingsField::Fullscreen => {
                    s.fullscreen = !s.fullscreen;
                    history::save_fullscreen(s.fullscreen);
                }
                SettingsField::SeqWrite => s.seq_write = !s.seq_write,
                SettingsField::SeqRead => s.seq_read = !s.seq_read,
                SettingsField::RandWrite => s.rand_write = !s.rand_write,
                SettingsField::RandRead => s.rand_read = !s.rand_read,
            },
            _ => {}
        }
    }

    pub fn tick(&mut self) {
        self.spinner_tick = self.spinner_tick.wrapping_add(1);

        let Some(rx) = self.rx.take() else { return };

        let mut finished = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                BenchMessage::TestStarted { test } => {
                    if let AppState::Running {
                        current_test,
                        progress,
                        live_mbps,
                        prev_bytes,
                        prev_elapsed,
                        ..
                    } = &mut self.state
                    {
                        *current_test = test;
                        *progress = 0.0;
                        *live_mbps = 0.0;
                        *prev_bytes = 0;
                        *prev_elapsed = Duration::ZERO;
                    }
                }
                BenchMessage::Progress {
                    bytes_done,
                    bytes_total,
                    elapsed,
                    ..
                } => {
                    if let AppState::Running { progress, live_mbps, prev_bytes, prev_elapsed, .. } = &mut self.state {
                        *progress = bytes_done as f64 / bytes_total as f64;
                        let dt = elapsed.saturating_sub(*prev_elapsed).as_secs_f64();
                        let db = bytes_done.saturating_sub(*prev_bytes) as f64;
                        if dt > 0.05 {
                            let instant = db / dt / (1024.0 * 1024.0);
                            // Smooth with the previous value to avoid jitter
                            *live_mbps = if *live_mbps > 0.0 {
                                *live_mbps * 0.3 + instant * 0.7
                            } else {
                                instant
                            };
                            *prev_bytes = bytes_done;
                            *prev_elapsed = elapsed;
                        }
                    }
                }
                BenchMessage::TestComplete { test, result } => {
                    if let AppState::Running { completed, .. } = &mut self.state {
                        completed.push((test, result));
                    }
                }
                BenchMessage::SuiteComplete => {
                    self.finish_with_results();
                    finished = true;
                }
                BenchMessage::Error(_) => {
                    self.finish_with_results();
                    finished = true;
                }
            }
        }

        if finished {
            self.cancel = None;
            self._worker = None;
        } else {
            self.rx = Some(rx);
        }
    }

    fn go_back_to_drives(&mut self) {
        let drives = drives::detect_drives();
        self.state = AppState::SelectDrive {
            drives,
            selected: 0,
        };
    }

    fn finish_with_results(&mut self) {
        if let AppState::Running { completed, .. } = &mut self.state {
            let results = std::mem::take(completed);
            if results.is_empty() {
                self.go_back_to_drives();
            } else {
                let drive_name = self
                    .display_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| self.display_path.display().to_string());
                self.history = history::save(&drive_name, &results);
                self.view = View::Benchmark;
                self.state = AppState::Complete { results };
            }
        }
    }

    fn start_benchmark(&mut self) {
        let enabled = self.settings.enabled_tests();
        if enabled.is_empty() {
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let path = self.test_path.clone();
        let size_mb = self.settings.test_size_mb;
        let cancel_clone = cancel.clone();
        let first_test = enabled[0];

        let handle = std::thread::spawn(move || {
            bench::run_suite(tx, path, size_mb, cancel_clone, Some(enabled));
        });

        self.state = AppState::Running {
            current_test: first_test,
            progress: 0.0,
            live_mbps: 0.0,
            prev_bytes: 0,
            prev_elapsed: Duration::ZERO,
            completed: Vec::new(),
        };
        self.rx = Some(rx);
        self.cancel = Some(cancel);
        self._worker = Some(handle);
    }

    fn cancel_benchmark(&mut self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        self.rx = None;
        self.cancel = None;
        if let Some(handle) = self._worker.take() {
            let _ = handle.join();
        }
    }
}
