use futures::future::LocalBoxFuture;
use tokio::sync::{mpsc, oneshot};

pub type FutureFactory = Box<dyn FnOnce() -> LocalBoxFuture<'static, ()> + Send>;

pub trait ThreadSpawn {
    /// Schedule a future to be spawned on a remote async thread, and return a handle for awaiting its
    /// completion.
    ///
    /// Notably, although the function that creates the future needs to be [`Send`], the future
    /// itself does not need to be [`Send`].
    fn spawn<Fut, T>(&self, f: impl FnOnce() -> Fut + Send + 'static) -> oneshot::Receiver<T>
    where
        T: Send + 'static,
        Fut: Future<Output = T> + 'static;
}

/// Object that manages the async runtime thread in the background.
///
/// Attempts to terminate the runtime on drop.
pub struct AsyncRuntime {
    mailbox: mpsc::Sender<FutureFactory>,
    _thread: Option<std::thread::JoinHandle<()>>,
}

impl AsyncRuntime {
    /// Start an async runtime in the background.
    pub fn start() -> Self {
        // build runtime and get handle
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("Failed to create tokio runtime!");

        let (tx, mut rx) = mpsc::channel::<FutureFactory>(10);

        // actually spawn the thread
        let thread = std::thread::Builder::new()
            .name("tokio".into())
            .spawn(move || {
                runtime.block_on(async move {
                    while let Some(task) = rx.recv().await {
                        tokio::task::spawn_local(task());
                    }
                })
            })
            .expect("Failed to spawn async runtime thread!");

        Self {
            mailbox: tx,
            _thread: Some(thread),
        }
    }
}

impl ThreadSpawn for AsyncRuntime {
    fn spawn<Fut, T>(&self, f: impl FnOnce() -> Fut + Send + 'static) -> oneshot::Receiver<T>
    where
        T: Send + 'static,
        Fut: Future<Output = T> + 'static,
    {
        let (tx, rx) = oneshot::channel();
        self.mailbox
            .blocking_send(Box::new(move || {
                let fut = f();
                let wrapped = async move {
                    tx.send(fut.await).ok();
                };
                Box::pin(wrapped)
            }))
            .expect("spawner dropped!");
        rx
    }
}
