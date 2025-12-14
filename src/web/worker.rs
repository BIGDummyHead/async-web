use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::{
    sync::{Mutex, mpsc::Sender},
    task::JoinHandle,
};

use crate::web::Queue;

///Takes a work Queue and works based on the queue, slowly consuming it.
pub struct Worker<R>
where
    R: Send + 'static,
{
    work: Arc<Queue<Pin<Box<dyn Future<Output = R> + 'static + Send>>>>,
    task: Option<JoinHandle<()>>,
    sender: Sender<R>,
    closed: Arc<Mutex<bool>>
}

impl<R> Worker<R>
where
    R: Send + 'static,
{
    pub fn new(
        sender: Sender<R>,
        work: Arc<Queue<Pin<Box<dyn Future<Output = R> + 'static + Send>>>>,
    ) -> Self {
        Self {
            sender,
            work,
            task: None,
            closed: Arc::new(Mutex::new(false))
        }
    }

    pub async fn start_worker(&mut self) -> () {
        if let Some(_) = &self.task {
            return;
        }

        let work = self.work.clone();
        let sender = self.sender.clone();
        let closed = self.closed.clone();

        let task = tokio::task::spawn(async move {
            while let Some(func) = work.deque(Some(closed.clone())).await {
                
                let v = func.await;
                let send_result = sender.send(v).await;

                if let Err(e) = send_result {
                    eprintln!("Error in sending data: {e}");
                }

            }
        });

        self.task = Some(task);
    }

    pub async fn close(&mut self) -> () {
        if let None = self.task  {
            return;
        }
        else if *self.closed.lock().await {
            return;
        }

        *self.closed.lock().await = true;
        self.work.deque_lock.notify_one();
        let j_result = self.task.as_mut().unwrap().await;
        
        if let Err(e) = j_result {
            eprintln!("Could not join task: {e}");
        }
    }
}
