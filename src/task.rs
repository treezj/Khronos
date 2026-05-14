use std::{
    pin::Pin,
    rc::Weak,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{RawWaker, RawWakerVTable, Waker},
};

use crate::task_manager::TaskManager;

static TASK_TAG_NUM: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct TaskTag(usize);

//representation of an asynchronous future to be executed.
pub struct Task {
    pub future: Mutex<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
    pub task_task: TaskTag,
    pub manager: Weak<TaskManager>,
    pub abort: Arc<AtomicBool>,
}

impl Task {
    const WAKER_VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    pub fn waker(self: Arc<Self>) -> Waker {
        let opaque_ptr = Arc::into_raw(self) as *const ();
        let vtable = &Self::WAKER_VTABLE;
        unsafe { Waker::from_raw(RawWaker::new(opaque_ptr, vtable)) }
    }
    pub fn generate_tag() -> TaskTag {
        TaskTag(TASK_TAG_NUM.fetch_add(1, Ordering::Relaxed))
    }
    pub fn has_aborted(&self) -> bool {
        self.abort.load(Ordering::SeqCst)
    }
}

fn clone(ptr: *const ()) -> RawWaker {
    let original: Arc<Task> = unsafe { Arc::from_raw(ptr as _) };
    let cloned = original.clone();
    std::mem::forget(original);
    std::mem::forget(cloned);
    RawWaker::new(ptr, &Task::WAKER_VTABLE)
}

fn drop(ptr: *const ()) {
    let _: Arc<Task> = unsafe { Arc::from_raw(ptr as _) };
}

fn wake(ptr: *const ()) {
    let task_ptr: Arc<Task> = unsafe { Arc::from_raw(ptr as _) };
    let task_manager = task_ptr.manager.upgrade().unwrap();
    task_manager.register_or_execute_non_blocking_task(task_ptr);
}

fn wake_by_ref(ptr: *const ()) {
    let task_ptr: Arc<Task> = unsafe { Arc::from_raw(ptr as _) };
    let task_manager = task_ptr.manager.upgrade().unwrap();
    task_manager.register_or_execute_non_blocking_task(Arc::clone(&task_ptr));
    std::mem::forget(task_ptr);
}
