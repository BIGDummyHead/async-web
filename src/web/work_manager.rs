use std::{pin::Pin, sync::Arc};

use futures::future::join_all;
use tokio::{
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    }
};

use crate::web::{Queue, Worker};


/// Represents a distrubutor of work.
pub struct WorkManager<R>
where
    R: Send + 'static,
{
    /// The amount of workers started on creation.
    size: usize,
    /// The sender to clone for the receiver
    sender: Sender<R>,
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
    /// Create a new set of n workers to complete work for this R set of functions. 
    /// 
    /// An optional buffer may be passed in for the mpsc::channel. This buffer controls the amount of messages the sender must receive before it is flushed
    /// This count is automatically set to 0 if "None" is passed in.
    pub async fn new(size: usize, opt_buffer: Option<usize>) -> Self {
        let buffer = match opt_buffer {
            None => 1,
            Some(x) => x,
        };

        let (tx, rx) = mpsc::channel(buffer);

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

    /// The amount of workers created
    pub fn worker_count(&self) -> usize {
        self.size
    }

    /// Get the queue of work being used by the workers.
    pub fn get_queue(&self) -> Arc<Queue<Pin<Box<dyn Future<Output = R> + Send + 'static>>>> {
        self.work.clone()
    }

    async fn create_workers(
        size: usize,
        sender: &Sender<R>,
        work: &Arc<Queue<Pin<Box<dyn Future<Output = R> + Send + 'static>>>>,
    ) -> Vec<Worker<R>> {
        
        let mut work_futs= vec![];

        for _ in 0..size {
            let tx = sender.clone();

            let wrk = work.clone();

            let mut worker = Worker::new(tx, wrk);

            //push an async closure that starts the worker then returns it... these are awaited later.
            work_futs.push(async move {
                //we can ignore this value.
                let _ = worker.start_worker().await;
                return worker;
            });
        }


        join_all(work_futs).await
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
