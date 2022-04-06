/// Very simple ring queue
/// /!\ only contains N - 1 elements max due to the design (% N)
pub struct Queue<T: Copy, const N: usize> {
    data: [T; N],
    head: u8,
    tail: u8,
}

impl<T: Copy, const N: usize> Queue<T, N> {
    pub fn new(data: [T; N]) -> Self {
        Self {
            data,
            head: 0u8,
            tail: 0u8,
        }
    }

    pub fn push(&mut self, value: T) {
        if self.is_full() {
            self.tail = (self.tail + 1) % (N as u8);
        }
        self.data[self.head as usize] = value;
        self.head = (self.head + 1) % (N as u8);
    }

    pub fn pop(&mut self) -> T {
        debug_assert!(self.size() > 0);
        let value = self.data[self.tail as usize];
        self.tail = (self.tail + 1) % (N as u8);
        value
    }

    pub fn is_full(&self) -> bool {
        (self.head.wrapping_sub(self.tail) % (N as u8)) == ((N - 1) as u8)
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn size(&self) -> u8 {
        self.head.wrapping_sub(self.tail) % (N as u8)
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_push_a_new_element() {
        let mut q = Queue::new([0u8; 16]);

        q.push(3);
        q.push(4);
        q.push(5);

        assert_eq!(q.data[0], 3);
        assert_eq!(q.data[1], 4);
        assert_eq!(q.data[2], 5);
    }

    #[test]
    fn it_pop_elements() {
        let mut q = Queue::new([0u8; 16]);

        q.push(3);
        q.push(4);
        q.push(5);

        assert_eq!(q.pop(), 3);
        assert_eq!(q.pop(), 4);
        assert_eq!(q.pop(), 5);
    }

    #[test]
    fn it_checks_size() {
        let mut q = Queue::new([0u8; 4]);

        assert_eq!(q.size(), 0);
        assert!(q.is_empty());
        q.push(3);
        assert_eq!(q.size(), 1);
        q.push(4);
        assert_eq!(q.size(), 2);
        q.push(5);
        assert_eq!(q.size(), 3);
        assert!(q.is_full());
        q.pop();
        assert_eq!(q.size(), 2);
        q.pop();
        assert_eq!(q.size(), 1);
        q.pop();
        assert_eq!(q.size(), 0);
    }
}
