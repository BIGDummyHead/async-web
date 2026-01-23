use std::{pin::Pin, sync::Arc};

use futures::future::join_all;
use tokio::sync::{
    Mutex,
    mpsc::{self, Receiver, Sender},
};

use crate::factory::{Queue, Worker};

/// # Work Manager
///
/// Represents a manager of [`Worker`]s. Contains a queue of work to be complete by the N workers.
///
pub struct WorkManager<R>
where
    R: Send + 'static,
{
    /// The amount of workers started on creation.
    size: usize,
    /// The sender to clone for the receiver
    pub sender: Sender<R>,
    ///The receiver, used to get incoming data from workers.
    pub receiver: Arc<Mutex<Receiver<R>>>,
    /// Vec of created workers
    workers: Vec<Worker<R>>,
    /// Work to complete. Async work that returns the R type given
    work: Arc<Queue<Pin<Box<dyn Future<Output = R> + Send + 'static>>>>,
}

impl<R> WorkManager<R>
where
    R: Send + 'static,
{
    /// # New
    ///
    /// Creates a new work manager that has N amount of workers.
    ///
    /// The amount of workers also sets the size of the channel buffer size.
    ///
    /// For example this allows us to distribute a batch of work.
    ///
    /// Assume that we make a WorkManager of 100 workers and 200 task come in, each worker will assume a task, run, finish, and take another task.
    pub async fn new(size: usize) -> Self {
        let (tx, rx) = mpsc::channel(size);

        let receiver = Arc::new(Mutex::new(rx));

        let work = Arc::new(Queue::new());

        let workers = Self::create_workers(size, &tx, &work).await;

        Self {
            size,
            sender: tx,
            receiver,
            workers,
            work,
        }
    }

    /// Returns the amount of workers this manager uses. NOT the size that was initially used.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    /// # Worker Errors
    ///
    /// Returns the count of errors that occurred when creating workers.
    pub fn worker_errors(&self) -> usize {
        self.size - self.worker_count()
    }

    /// # create workers
    ///
    /// Creates a batch of workers Of the size, cloning both the sender and the work load references.
    ///
    /// It is important to note that if the worker upon creation experiences an error it is not captured. And the reference is dropped.
    async fn create_workers(
        size: usize,
        sender: &Sender<R>,
        work: &Arc<Queue<Pin<Box<dyn Future<Output = R> + Send + 'static>>>>,
    ) -> Vec<Worker<R>> {
        // work start futures
        let mut work_futs = vec![];

        // for the size of workers
        for _ in 0..size {
            //clone the sender
            let data_sender = sender.clone();

            //clone the work queue
            let work_queue = work.clone();

            let mut worker = Worker::new(data_sender, work_queue);

            //push each worker future and map the result to return the Worker that was created.
            work_futs.push(async move {
                //we can ignore this value.
                let start_result = worker.start_worker().await.map(|_| worker);
                start_result
            });
        }

        //join all futures, run, into iter, filter into map where results are ok, collect into Vec<Worker<R>>
        join_all(work_futs)
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect()
    }

    /// Add work to the queue for workers to complete.
    pub async fn add_work(&self, work: Pin<Box<dyn Future<Output = R> + Send + 'static>>) -> () {
        self.work.queue(work).await
    }

    /// Close all workers, the queue, and wait for them to finish
    pub async fn close_and_finish_work(&mut self) -> () {
        let mut close_futs = vec![];

        for worker in &mut self.workers {
            let close_fut = worker.close();
            close_futs.push(close_fut);
        }

        join_all(close_futs).await;
    }
}
