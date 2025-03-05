use std::{
    cmp::Ordering,
    collections::{binary_heap::PeekMut, BinaryHeap},
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
struct Entry<T> {
    idx: u64,
    timestamp_ms: u64,
    arrival_time: Instant,
    value: T,
}

impl<T> Ord for Entry<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.idx.cmp(&other.idx).reverse()
    }
}

impl<T> PartialOrd for Entry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Eq for Entry<T> {}

impl<T> PartialEq for Entry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx.eq(&other.idx)
    }
}

pub struct Buffer<T> {
    entries: BinaryHeap<Entry<T>>,
    last_entry: Option<Entry<()>>,
    last_popped_idx: u64,
    jitter_ms: i64,
    buf_depth: Duration,
    min_depth: Duration,
    max_depth: Duration,
}

impl<T> Buffer<T> {
    pub fn new(min_depth: Duration, max_depth: Duration) -> Self {
        assert!(min_depth <= max_depth);

        Self {
            entries: BinaryHeap::new(),
            last_entry: None,
            last_popped_idx: 0,
            jitter_ms: 0,
            buf_depth: min_depth,
            min_depth,
            max_depth,
        }
    }

    pub fn insert(&mut self, idx: u64, timestamp_ms: u64, value: T) {
        let entry = Entry {
            idx,
            timestamp_ms,
            arrival_time: Instant::now(),
            value: (),
        };

        if let Some(last_entry) = &self.last_entry {
            let delay = entry.arrival_time.duration_since(last_entry.arrival_time);
            let expected_delay_ms = entry.timestamp_ms as i64 - last_entry.timestamp_ms as i64;

            self.jitter_ms +=
                ((delay.as_millis() as i64 - expected_delay_ms) - self.jitter_ms) / 16;
            self.buf_depth = if self.jitter_ms > 0 {
                self.buf_depth
                    .saturating_add(Duration::from_millis(self.jitter_ms as u64))
            } else {
                self.buf_depth
                    .saturating_sub(Duration::from_millis((-self.jitter_ms) as u64))
            }
            .clamp(self.min_depth, self.max_depth);
        }
        self.last_entry = Some(entry);

        // Discard late packets
        if idx <= self.last_popped_idx {
            return;
        }

        self.entries.push(Entry {
            idx: entry.idx,
            timestamp_ms: entry.timestamp_ms,
            arrival_time: entry.arrival_time,
            value,
        });
    }

    pub fn pop(&mut self) -> Output<T> {
        let now = Instant::now();
        let mut data = Vec::new();
        while let Some(entry) = self.entries.peek_mut() {
            let pkt_ready = entry.arrival_time + self.buf_depth;
            if let Some(wait_time) = pkt_ready.checked_duration_since(now) {
                return Output { wait_time, data };
            }

            let entry = PeekMut::pop(entry);
            self.last_popped_idx = entry.idx;
            data.push(entry.value);
        }

        Output {
            wait_time: self.buf_depth,
            data,
        }
    }
}

pub struct Output<T> {
    pub wait_time: Duration,
    pub data: Vec<T>,
}
