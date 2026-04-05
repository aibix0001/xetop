/// Fixed-capacity ring buffer for sparkline history.
#[derive(Clone)]
pub struct RingBuffer {
    data: Vec<u64>,
    capacity: usize,
    write_pos: usize,
    len: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            data: vec![0; capacity],
            capacity,
            write_pos: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, value: f64) {
        // Sparkline expects u64; scale percentage to 0-100 range.
        self.data[self.write_pos] = value.clamp(0.0, 100.0) as u64;
        self.write_pos = (self.write_pos + 1) % self.capacity;
        if self.len < self.capacity {
            self.len += 1;
        }
    }

    /// Return data in chronological order for sparkline rendering.
    pub fn to_vec(&self) -> Vec<u64> {
        if self.len < self.capacity {
            self.data[..self.len].to_vec()
        } else {
            let mut result = Vec::with_capacity(self.capacity);
            result.extend_from_slice(&self.data[self.write_pos..]);
            result.extend_from_slice(&self.data[..self.write_pos]);
            result
        }
    }
}
