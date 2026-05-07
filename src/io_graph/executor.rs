use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{Scope, ScopedJoinHandle},
};

use crate::io_graph::{Node, Worker};

pub struct GraphBuilder<'scope, 'env> {
    start_condvar: Condvar,
    start_mutex: &'scope Mutex<Option<Arc<GraphContext>>>,
    scope: &'scope Scope<'scope, 'env>,
}

impl<'scope, 'env> GraphBuilder<'scope, 'env>
where
    'env: 'scope,
{
    pub fn add_node(&self, node: &impl Node<'env>) {}

    pub fn add_worker<W>(
        &'scope self,
        worker: Box<W>,
    ) -> ScopedJoinHandle<'scope, Result<W::Output, std::io::Error>>
    where
        W: Worker<'env>,
    {
        self.scope.spawn(move || {
            let ctx = loop {
                let guard = self.start_mutex.lock().unwrap();
                if let Some(x) = guard.clone() {
                    break x;
                }
                self.start_condvar.wait(guard).unwrap();
            };
            worker.run(&ctx)
        })
    }

    pub fn start(&'scope self) {
        *self.start_mutex.lock().unwrap() = Some(Arc::new(GraphContext::new()));
    }
}

pub struct GraphContext {
    halt: AtomicBool,
}

impl GraphContext {
    pub fn new() -> Self {
        Self { halt: false.into() }
    }

    pub fn halt(&self) -> bool {
        self.halt.load(Ordering::Relaxed)
    }
}
