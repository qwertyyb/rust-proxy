pub mod socks;
pub mod utils;

use std::{sync::{mpsc::{Sender, self, Receiver}, Mutex, Arc}, thread::{JoinHandle, self}};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
  thread: Option<JoinHandle<()>>
}

impl Worker {
  fn new(receiver: Arc<Mutex<Receiver<Job>>>) -> Self {
    Worker {
      thread: Some(thread::spawn(move || loop {
        let job = receiver.lock().unwrap().recv();
        if let Ok(job) = job {
          job();
        } else {
          break;
        };
      }))
    }
  }
}

impl Drop for Worker {
  fn drop(&mut self) {
    if let Some(thread) = self.thread.take() {
      let _ = thread.join();
    }
  }
}

pub struct ThreadPool {
  workers: Vec<Worker>,
  sender: Option<Sender<Job>>
}

impl ThreadPool {
  pub fn with_capacity(size: usize) -> Self {
    let (sender, receiver) = mpsc::channel();

    let receiver = Arc::new(Mutex::new(receiver));
    let mut workers = Vec::new();
    for _i in 0..size {
      workers.push(Worker::new(Arc::clone(&receiver)));
    }

    Self { sender: Some(sender), workers }
  }

  pub fn run<F>(&self, f: F)
    where
      F: FnOnce() + Send + 'static 
    {
    let job = Box::new(f);
    let _ = self.sender.as_ref().unwrap().send(job);
  }
}

impl Drop for ThreadPool {
  fn drop(&mut self) {
      self.sender.take();
      self.workers.clear();
  }
}
