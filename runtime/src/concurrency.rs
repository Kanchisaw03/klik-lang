// KLIK Runtime - Concurrency primitives
// Task spawning, channels, mutexes

use std::future::Future;
use std::pin::Pin;

/// A handle to a spawned task
pub struct TaskHandle<T> {
    inner: tokio::task::JoinHandle<T>,
}

impl<T: Send + 'static> TaskHandle<T> {
    pub async fn join(self) -> Result<T, TaskError> {
        self.inner
            .await
            .map_err(|e| TaskError::JoinError(e.to_string()))
    }
}

/// Spawn a new async task
pub fn spawn_task<F, T>(future: F) -> TaskHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let handle = tokio::spawn(future);
    TaskHandle { inner: handle }
}

/// Errors from task operations
#[derive(Debug)]
pub enum TaskError {
    JoinError(String),
    Cancelled,
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::JoinError(msg) => write!(f, "task join error: {}", msg),
            TaskError::Cancelled => write!(f, "task cancelled"),
        }
    }
}

impl std::error::Error for TaskError {}

/// Bounded channel for inter-task communication
pub struct Channel<T> {
    sender: tokio::sync::mpsc::Sender<T>,
    receiver: Option<tokio::sync::mpsc::Receiver<T>>,
}

impl<T: Send + 'static> Channel<T> {
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity);
        Self {
            sender,
            receiver: Some(receiver),
        }
    }

    pub fn sender(&self) -> ChannelSender<T> {
        ChannelSender {
            inner: self.sender.clone(),
        }
    }

    pub fn take_receiver(&mut self) -> Option<ChannelReceiver<T>> {
        self.receiver.take().map(|r| ChannelReceiver { inner: r })
    }
}

pub struct ChannelSender<T> {
    inner: tokio::sync::mpsc::Sender<T>,
}

impl<T: Send> ChannelSender<T> {
    pub async fn send(&self, value: T) -> Result<(), T> {
        self.inner.send(value).await.map_err(|e| e.0)
    }
}

pub struct ChannelReceiver<T> {
    inner: tokio::sync::mpsc::Receiver<T>,
}

impl<T> ChannelReceiver<T> {
    pub async fn recv(&mut self) -> Option<T> {
        self.inner.recv().await
    }
}

/// Mutex wrapper for KLIK runtime
pub struct KlikMutex<T> {
    inner: parking_lot::Mutex<T>,
}

impl<T> KlikMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: parking_lot::Mutex::new(value),
        }
    }

    pub fn lock(&self) -> parking_lot::MutexGuard<'_, T> {
        self.inner.lock()
    }

    pub fn try_lock(&self) -> Option<parking_lot::MutexGuard<'_, T>> {
        self.inner.try_lock()
    }
}

/// Type alias for boxed futures
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
