use std::{
    cmp::Ordering,
    collections::{binary_heap::PeekMut, BinaryHeap},
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
struct Entry<T> {
    idx: u64,
    arrival_time: Instant,
    timestamp: Duration,
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

impl<T> Entry<T> {
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Entry<U> {
        Entry {
            idx: self.idx,
            arrival_time: self.arrival_time,
            timestamp: self.timestamp,
            value: f(self.value),
        }
    }
}

struct Inner<T> {
    entries: BinaryHeap<Entry<T>>,
    last_entry: Option<Entry<()>>,
    last_popped_idx: u64,
    buf_depth: Duration,
    jitter: Duration,
}

pub struct Buffer<T> {
    inner: Mutex<Inner<T>>,
    min_depth: Duration,
    max_depth: Duration,
}

impl<T> Buffer<T> {
    pub fn new(min_depth: Duration, max_depth: Duration) -> Self {
        assert!(min_depth <= max_depth);

        Self {
            inner: Mutex::new(Inner {
                entries: BinaryHeap::new(),
                last_entry: None,
                last_popped_idx: 0,
                buf_depth: min_depth,
                jitter: Duration::ZERO,
            }),

            min_depth,
            max_depth,
        }
    }

    // TODO : replace Duration with samples mb
    pub fn insert(&self, idx: u64, timestamp: Duration, value: T) {
        let this = &mut *self.inner.lock().unwrap();
        let entry = Entry {
            idx,
            arrival_time: Instant::now(),
            timestamp,
            value: (),
        };

        if let Some(last_entry) = &this.last_entry {
            let delay = entry.arrival_time.duration_since(last_entry.arrival_time);
            let expected_delay = entry.timestamp - last_entry.timestamp;
            this.jitter += (delay - expected_delay - this.jitter) / 16;
            this.buf_depth = (this.buf_depth + this.jitter).clamp(self.min_depth, self.max_depth);
        }
        this.last_entry = Some(entry);

        // Discard late packets
        if idx <= this.last_popped_idx {
            return;
        }

        this.entries.push(entry.map(|()| value));
    }

    pub fn pop(&self) -> Output<T> {
        let this = &mut *self.inner.lock().unwrap();

        let now = Instant::now();
        let mut data = Vec::new();
        while let Some(entry) = this.entries.peek_mut() {
            let pkt_ready = entry.arrival_time + this.buf_depth;
            if let Some(wait_time) = pkt_ready.checked_duration_since(now) {
                return Output { wait_time, data };
            }

            let entry = PeekMut::pop(entry);
            this.last_popped_idx = entry.idx;
            data.push(entry.value);
        }

        Output {
            wait_time: this.buf_depth,
            data,
        }
    }
}

pub struct Output<T> {
    pub wait_time: Duration,
    pub data: Vec<T>,
}
