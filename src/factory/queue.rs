use std::sync::Arc;

use tokio::sync::{Mutex, Notify};


/// ## Queue
/// 
/// Async-safe Queue used for evenly waiting and distributing workloads. 
/// 
/// Type R of work is added to the queue, then the dequeu function is used to await for work.
/// 
/// ## Example
/// 
/// ```
/// let work_load = Queue::new();
/// 
/// work_load.queue(100);
/// 
/// //--snip--
/// 
/// //assume that we are in spawned task (one of many)
/// 
/// //we may also pass in an optional Arc<Mutex<bool>> that indicates to stop checking for values
/// let opt_value = work_load_clone.deque(None);
/// 
/// ```
pub struct Queue<R> {
    work: Mutex<Vec<R>>,
    pub deque_lock: Notify
}

/// Async based Queue
impl<R> Queue<R> {

    /// Create a new queue
    pub fn new() -> Self {
        Self { work: Mutex::new(Vec::new()), deque_lock: Notify::new() }
    }

    /// Queue a value
    pub async fn queue(&self, value: R) -> () {
        let mut work = self.work.lock().await;

        work.push(value);
        self.deque_lock.notify_one();
    }

    async fn try_deque(&self) -> Option<R> {
        let mut locked_queue = self.work.lock().await;

        if locked_queue.is_empty() {
            return None;
        }

        Some(locked_queue.remove(0))
    }

    /// Deque and wait for a value.
    /// 
    /// Returns None if there was a closure
    pub async fn deque(&self, closure: Option<Arc<Mutex<bool>>>) -> Option<R> {

        let fut = self.deque_lock.notified();
        tokio::pin!(fut);

        loop {

            if let Some(c) = &closure {
                if *c.lock().await {
                    return None;
                }
            }

            fut.as_mut().enable();

            if let Some(r) = self.try_deque().await {
                return Some(r);
            }

            fut.as_mut().await;

            fut.set(self.deque_lock.notified());
        }
    }

}

