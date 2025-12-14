use std::sync::Arc;

use tokio::sync::{Mutex, Notify};


/// An async queue for work
pub struct Queue<R> {
    work: Mutex<Vec<R>>,
    pub deque_lock: Notify
}

impl<R> Queue<R> {

    pub fn new() -> Self {
        Self { work: Mutex::new(Vec::new()), deque_lock: Notify::new() }
    }

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

