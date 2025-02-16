use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
struct Entry<T> {
    arrival_time: Instant,
    timestamp: Duration,
    value: T,
}

impl<T> Entry<T> {
    fn map<U>(self, f: impl FnOnce(T) -> U) -> Entry<U> {
        Entry {
            arrival_time: self.arrival_time,
            timestamp: self.timestamp,
            value: f(self.value),
        }
    }
}

pub struct Buffer<T> {
    // TODO : use set and hash/eq by number inside of struct
    entries: BTreeMap<u64, Entry<T>>,
    last_entry: Option<Entry<()>>,
    buf_depth: Duration,
    min_depth: Duration,
    max_depth: Duration,
    jitter: Duration,
}

impl<T> Buffer<T> {
    pub fn new(min_depth: Duration, max_depth: Duration) -> Self {
        Self {
            entries: BTreeMap::new(),
            last_entry: None,
            buf_depth: min_depth,
            min_depth,
            max_depth,
            jitter: Duration::ZERO,
        }
    }

    // TODO : replace Duration with samples mb
    pub fn insert(&mut self, idx: u64, timestamp: Duration, value: T) {
        let entry = Entry {
            arrival_time: Instant::now(),
            timestamp,
            value: (),
        };
        if let Some(last_entry) = &self.last_entry {
            let delay = entry.arrival_time.duration_since(last_entry.arrival_time);
            let expected_delay = entry.timestamp - last_entry.timestamp;
            self.jitter += ((delay - expected_delay) - self.jitter) / 16;
            self.buf_depth = (self.buf_depth + self.jitter).clamp(self.min_depth, self.max_depth);
        }
        self.last_entry = Some(entry);

        // TODO : GC too old packets
        self.entries.insert(idx, entry.map(|_| value));
    }

    pub fn pop(&mut self) -> Output<T> {
        let Some((_, pkt)) = self.entries.pop_first() else {
            return Output::EmptyBuffer;
        };

        let now = Instant::now();
        let pkt_ready = pkt.arrival_time + self.buf_depth;
        match pkt_ready.checked_duration_since(now) {
            Some(wait_time) => Output::NotYet { wait_time },
            None => match (pkt_ready + self.buf_depth).checked_duration_since(now) {
                Some(latency) => Output::TooLate { latency },
                None => Output::Value(pkt.value),
            },
        }
    }
}

pub enum Output<T> {
    EmptyBuffer,
    NotYet { wait_time: Duration },
    TooLate { latency: Duration },
    Value(T),
}
