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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer() {
        let buf = RingBuffer::new(5);
        assert_eq!(buf.to_vec(), Vec::<u64>::new());
    }

    #[test]
    fn partial_fill() {
        let mut buf = RingBuffer::new(5);
        buf.push(10.0);
        buf.push(20.0);
        buf.push(30.0);
        assert_eq!(buf.to_vec(), vec![10, 20, 30]);
    }

    #[test]
    fn full_wrap() {
        let mut buf = RingBuffer::new(3);
        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0);
        buf.push(4.0); // overwrites 1
        assert_eq!(buf.to_vec(), vec![2, 3, 4]);
    }

    #[test]
    fn clamps_to_100() {
        let mut buf = RingBuffer::new(3);
        buf.push(150.0);
        buf.push(-10.0);
        assert_eq!(buf.to_vec(), vec![100, 0]);
    }

    #[test]
    fn zero_capacity_guarded() {
        let mut buf = RingBuffer::new(0);
        buf.push(42.0);
        assert_eq!(buf.to_vec(), vec![42]);
    }
}
