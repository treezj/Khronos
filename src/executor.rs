use std::{
    sync::{
        Arc,
        mpsc::{Receiver, SyncSender},
    },
    task::{Context, Waker},
};

use crate::{task::Task, task_manager::TaskManager};
pub type BlockingFn = dyn FnOnce() + Send + 'static;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct ExecutorId(usize);

impl ExecutorId {
    pub fn get(&self) -> usize {
        self.0
    }
}

pub enum ExecutorTask {
    Task(Arc<Task>),
    Finished,
}

pub struct Executor {
    ready_queue: Receiver<ExecutorTask>,
    panic_tx: SyncSender<()>,
    id: ExecutorId,
}

pub enum Status {
    Awaited(Waker),
    Happened
}

impl Executor {
    pub fn new(rx: Receiver<ExecutorTask>, executor_id: usize, panic_tx: SyncSender<()>) -> Self {
        Self {
            ready_queue: rx,
            panic_tx,
            id: ExecutorId(executor_id),
        }
    }

    pub fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            match task {
                ExecutorTask::Finished => return,
                ExecutorTask::Task(task) => self.forward_task(task),
            };
            let tm = TaskManager::get();
            tm.executor_ready(self.id());
        }
    }
    fn forward_task(&self, task: Arc<Task>) {
        let mut future = task.future.lock().unwrap();

        let waker = Arc::clone(&task).waker();
        let mut context = Context::from_waker(&waker);

        if let Err(e) = std::panic::catch_unwind(move || future.as_mut().poll(&mut context)) {
            println!("EXECUTOR PANIC FUNCTION. ERROR {:?}", e);
            self.panic_tx.send(()).unwrap();
        }
    }
    pub fn id(&self) -> ExecutorId {
        self.id
    }
}
