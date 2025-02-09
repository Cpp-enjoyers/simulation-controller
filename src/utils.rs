#![allow(clippy::len_without_is_empty)]
use std::collections::VecDeque;

/// A simple event queue that stores the last `capacity` events.
pub struct EventQueue<T> {
    queue: VecDeque<T>,
    capacity: usize,
}

impl<T> EventQueue<T> {
    /// Create a new event queue with the given capacity.
    #[must_use] pub fn new(capacity: usize) -> Self {
        EventQueue {
            queue: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Push a new event to the queue. If the queue is full, the oldest event will be removed.
    pub fn push(&mut self, event: T) {
        if self.queue.len() == self.capacity {
            self.queue.pop_front();
        }
        self.queue.push_back(event);
    }

    /// Get all events in the queue.
    #[must_use] pub fn get(&self) -> Vec<&T> {
        self.queue.iter().collect()
    }

    /// Get the number of events in the queue.
    #[must_use] pub fn len(&self) -> usize {
        self.queue.len()
    }
}