use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

// Increment counter by one until this SessionCounter is dropped
pub struct SessionCounter {
    counter: Arc<AtomicU64>,
}

impl SessionCounter {
    pub fn new(counter: Arc<AtomicU64>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);

        SessionCounter { counter }
    }
}

impl Drop for SessionCounter {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}
