use crossbeam_channel::{bounded, Receiver, Sender};
use std::time::{Duration, Instant};

pub struct PtyCoalescer {
    buffer: Vec<u8>,
    tx: Sender<Vec<u8>>,
    window: Duration,
    last_flush: Instant,
    max_batch_size: usize,
}

impl PtyCoalescer {
    pub fn new(window: Duration) -> (Self, Receiver<Vec<u8>>) {
        let (tx, rx) = bounded(128);
        let coalescer = Self {
            buffer: Vec::with_capacity(8192),
            tx,
            window,
            last_flush: Instant::now(),
            max_batch_size: 65536, // 64 KB
        };
        (coalescer, rx)
    }

    pub fn push_bytes(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
        if self.buffer.len() >= self.max_batch_size || self.last_flush.elapsed() >= self.window {
            self.flush();
        }
    }

    pub fn flush(&mut self) {
        if !self.buffer.is_empty() {
            let data = std::mem::replace(&mut self.buffer, Vec::with_capacity(8192));
            let _ = self.tx.send(data);
            self.last_flush = Instant::now();
        }
    }

    pub fn maybe_flush(&mut self) {
        if !self.buffer.is_empty() && self.last_flush.elapsed() >= self.window {
            self.flush();
        }
    }
}
