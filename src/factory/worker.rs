use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::{
    sync::{Mutex, mpsc::Sender},
    task::JoinHandle,
};


use crate::{factory::Queue, web::errors::WorkerError};

/// # Worker <R>
///
/// A worker that dequeues a piece of work in asynchronous manner, calling, finishing the task, and sends the data back to the sender.
pub struct Worker<R>
where
    R: Send + 'static,
{
    work: Arc<Queue<Pin<Box<dyn Future<Output = R> + 'static + Send>>>>,
    task: Option<JoinHandle<()>>,
    sender: Sender<R>,
    closed: Arc<Mutex<bool>>,
}

impl<R> Worker<R>
where
    R: Send + 'static,
{
    /// # New
    ///
    /// Creates a new worker with an output (Sender<R> of some R data) and queue of work that contains functions that output R
    pub fn new(
        sender: Sender<R>,
        work: Arc<Queue<Pin<Box<dyn Future<Output = R> + 'static + Send>>>>,
    ) -> Self {
        Self {
            sender,
            work,
            task: None,
            closed: Arc::new(Mutex::new(false)),
        }
    }

    /// # Start Worker
    ///
    /// Starts the worker, using the queued list of work to complete.
    ///
    /// May return a `WorkerError` if the task is already running.
    pub async fn start_worker(&mut self) -> Result<(), WorkerError> {
        // the worker was already started.
        if self.task.is_some() {
            return WorkerError::AlreadyRunning.into();
        }

        //refs to send
        let work = self.work.clone();
        let sender = self.sender.clone();
        let closed = self.closed.clone();

        //spawn a new task
        let task = tokio::task::spawn(async move {
            // while some work, send the "closed" flag into the work so we can ensure concurrency in ensuring workers do not keep working.
            //pass the closed ref to the deque func
            while let Some(func) = work.deque(Some(closed.clone())).await {
                //call and await the future, then send the result
                let func_result = func.await;
                let send_result = sender.send(func_result).await;

                //the channel was closed.
                if send_result.is_err() {
                    break;
                }
            }
        });

        self.task = Some(task);

        Ok(())
    }

    /// # Close
    ///
    /// Closes the worker, it does so by setting the closed flag to true, then joining the ongoing task.
    ///
    /// It is important to note that you may receive a Worker Error from the function if:
    ///
    /// * No Task is Running - NoTaskRunning
    /// * Already Closed - AlreadyClosed
    /// * The ongoing Task Fails to Join - TaskJoinFailure
    pub async fn close(&mut self) -> Result<(), WorkerError> {
        if let None = self.task {
            return WorkerError::NoTaskRunning.into();
        }

        let mut running_guard = self.closed.lock().await;

        if *running_guard {
            return Err(WorkerError::AlreadyClosed.into());
        }

        *running_guard = true;
        drop(running_guard);

        self.work.deque_lock.notify_one();

        let task = self.task.as_mut();

        if let None = task {
            return Ok(());
        }

        task.unwrap()
            .await
            .map_err(|_| WorkerError::TaskJoinFailure)?;

        Ok(())
    }
}
