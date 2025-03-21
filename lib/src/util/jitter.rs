use std::{
    cmp::Ordering,
    collections::{binary_heap::PeekMut, BinaryHeap},
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
struct Entry<T> {
    idx: u64,
    timestamp_ns: u128,
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
    jitter_ns: i128,
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
            jitter_ns: 0,
            buf_depth: min_depth,
            min_depth,
            max_depth,
        }
    }

    pub fn insert(&mut self, idx: u64, timestamp_ns: u128, value: T) {
        let entry = Entry {
            idx,
            timestamp_ns,
            arrival_time: Instant::now(),
            value: (),
        };

        if let Some(last_entry) = &self.last_entry {
            let delay = entry.arrival_time.duration_since(last_entry.arrival_time);
            let expected_delay_ns = entry.timestamp_ns as i128 - last_entry.timestamp_ns as i128;

            self.jitter_ns +=
                ((delay.as_millis() as i128 - expected_delay_ns) - self.jitter_ns) / 16;

            self.buf_depth = if self.jitter_ns > 0 {
                self.buf_depth
                    .saturating_add(Duration::from_nanos(self.jitter_ns.unsigned_abs() as u64))
            } else {
                self.buf_depth
                    .saturating_sub(Duration::from_nanos(self.jitter_ns.unsigned_abs() as u64))
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
            timestamp_ns: entry.timestamp_ns,
            arrival_time: entry.arrival_time,
            value,
        });
    }

    pub fn pop(&mut self) -> (Duration, Vec<T>) {
        let now = Instant::now();
        let mut data = Vec::new();
        while let Some(entry) = self.entries.peek_mut() {
            let pkt_ready = entry.arrival_time + self.buf_depth;
            if let Some(wait_time) = pkt_ready.checked_duration_since(now) {
                return (wait_time, data);
            }

            let entry = PeekMut::pop(entry);
            self.last_popped_idx = entry.idx;
            data.push(entry.value);
        }

        (self.buf_depth, data)
    }

    pub fn pop_remaining(self) -> Vec<T> {
        self.entries.into_iter().map(|e| e.value).collect()
    }
}
