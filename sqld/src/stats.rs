use std::fs::{File, OpenOptions};
use std::io::Seek;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Stats {
    inner: Arc<StatsInner>,
}

#[derive(Serialize, Deserialize, Default)]
struct StatsInner {
    rows_written: AtomicUsize,
    rows_read: AtomicUsize,
}

impl Stats {
    pub fn new(db_path: &Path) -> anyhow::Result<Self> {
        let stats_path = db_path.join("stats.json");
        let stats_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(stats_path)?;

        let stats_inner =
            serde_json::from_reader(&stats_file).unwrap_or_else(|_| StatsInner::default());
        let inner = Arc::new(stats_inner);

        spawn_stats_persist_thread(inner.clone(), stats_file);

        Ok(Self { inner })
    }

    /// increments the number of written rows by n
    pub fn inc_rows_written(&self, n: usize) {
        self.inner.rows_written.fetch_add(n, Ordering::Relaxed);
    }

    /// increments the number of read rows by n
    pub fn inc_rows_read(&self, n: usize) {
        self.inner.rows_read.fetch_add(n, Ordering::Relaxed);
    }

    /// returns the total number of rows read since this database was created
    pub fn rows_read(&self) -> usize {
        self.inner.rows_read.load(Ordering::Relaxed)
    }

    /// returns the total number of rows written since this database was created
    pub fn rows_written(&self) -> usize {
        self.inner.rows_written.load(Ordering::Relaxed)
    }
}

fn spawn_stats_persist_thread(stats: Arc<StatsInner>, mut file: File) {
    std::thread::spawn(move || loop {
        if file.rewind().is_ok() {
            let _ = serde_json::to_writer(&mut file, &stats);
        }
        std::thread::sleep(Duration::from_secs(5));
    });
}
