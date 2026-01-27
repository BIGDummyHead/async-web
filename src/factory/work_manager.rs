use std::{pin::Pin, sync::Arc};

use futures::future::join_all;
use tokio::sync::{
    Mutex,
    mpsc::{self, Receiver, Sender},
};

use crate::factory::{Queue, Worker, queue::QueueState, worker};

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
    pub async fn new(init_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(init_size);

        let receiver = Arc::new(Mutex::new(rx));

        let work = Arc::new(Queue::new());

        let workers = Self::create_workers(init_size, &tx, &work).await;

        Self {
            size: init_size,
            sender: tx,
            receiver,
            workers,
            work,
        }
    }

    /// # create workers
    ///
    /// Creates a batch of workers Of the size, cloning both the sender and the work load references.
    ///
    /// It is important to note that if the worker upon creation experiences an error it is not captured. And the reference is dropped.
    async fn create_workers(
        worker_count: usize,
        data_send: &Sender<R>,
        work_load: &Arc<Queue<Pin<Box<dyn Future<Output = R> + Send + 'static>>>>,
    ) -> Vec<Worker<R>> {
        // work start futures
        let mut work_futs = vec![];

        // for the size of workers
        for _ in 0..worker_count {
            //clone the sender
            let data_sender = data_send.clone();

            //clone the work queue
            let work_queue = work_load.clone();

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

    /// # queue work
    /// 
    /// Queues work with the given future.
    pub async fn queue_work(&self, work: Pin<Box<dyn Future<Output = R> + Send + 'static>>) -> QueueState {
        self.work.queue(work).await
    }


    /// # scale workers
    /// 
    /// Scales the worker count by the given factor.
    /// 
    /// For example, if the current workers are set to a size of 10 and the scale factor is 10
    /// 
    /// 90 workers are created, started, and set to the worker Vec.
    pub async fn scale_workers(&mut self, scale_factor: usize) -> () {

        //sizes and scalers.
        let current_size = self.size;
        let new_size = current_size * scale_factor;

        //create new workers with the difference.
        let mut new_workers = Self::create_workers(new_size - current_size, &self.sender, &self.work).await;

        //move the workers from one container to another.
        let mut worker_container = Vec::with_capacity(new_size);
        worker_container.append(&mut self.workers);
        worker_container.append(&mut new_workers);

        //set the new workers.
        self.size = worker_container.len();
        self.workers = worker_container;
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

    /// # size
    /// 
    /// Returns the size of current workers.
    pub fn size(&self) -> usize {
        self.size
    }
}
