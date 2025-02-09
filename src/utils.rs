use std::collections::VecDeque;


pub struct EventQueue<T> {
    queue: VecDeque<T>,
    capacity: usize,
}

impl<T> EventQueue<T> {
    pub fn new(capacity: usize) -> Self {
        EventQueue {
            queue: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, event: T) {
        if self.queue.len() == self.capacity {
            self.queue.pop_front();
        }
        self.queue.push_back(event);
    }

    pub fn get(&self) -> Vec<&T> {
        self.queue.iter().collect()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}