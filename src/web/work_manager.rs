use std::{pin::Pin, sync::Arc};

use futures::future::join_all;
use tokio::{
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
    task::{self, JoinHandle},
};

use crate::web::{Queue, Worker};

pub struct WorkManager<R>
where
    R: Send + 'static,
{
    size: usize,
    sender: Sender<R>,
    pub receiver: Arc<Mutex<Receiver<R>>>,
    workers: Vec<Worker<R>>,
    work: Arc<Queue<Pin<Box<dyn Future<Output = R> + Send + 'static>>>>,
}

impl<R> WorkManager<R>
where
    R: Send + 'static,
{
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

    pub fn worker_count(&self) -> usize {
        self.size
    }

    pub fn get_queue(&self) -> Arc<Queue<Pin<Box<dyn Future<Output = R> + Send + 'static>>>> {
        self.work.clone()
    }

    async fn create_workers(
        size: usize,
        sender: &Sender<R>,
        work: &Arc<Queue<Pin<Box<dyn Future<Output = R> + Send + 'static>>>>,
    ) -> Vec<Worker<R>> {
        
        let mut created_workers: Vec<Worker<R>> = vec![];
        for _ in 0..size {
            let tx = sender.clone();

            let wrk = work.clone();

            let mut worker = Worker::new(tx, wrk);

            worker.start_worker().await;

            created_workers.push(worker);
        }

        return created_workers;
    }


    pub async fn add_work(&self, work: Pin<Box<dyn Future<Output = R> + Send + 'static>>) -> () {
        self.work.queue(work).await
    }


    pub async fn close_and_finish_work(&mut self) -> () {

        let mut close_futs = vec![];

        for worker in &mut self.workers {
            let close_fut = worker.close();
            close_futs.push(close_fut);
        }

        println!("Waiting for {} to close", close_futs.len());
        join_all(close_futs).await;
    }

}
