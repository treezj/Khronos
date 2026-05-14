use std::{
    mem::MaybeUninit,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
pub struct LockFreeBoundedRingBuffer<T> {
    buffer: Vec<MaybeUninit<T>>,
    start: AtomicUsize,
    end: AtomicUsize,
    count: AtomicUsize,
}

impl<T> LockFreeBoundedRingBuffer<T> {
    const DEFAULT_BUFFER_SIZE: usize = 1024 * 1024;
    pub fn new(bound: usize) -> Self {
        Self {
            buffer: (0..bound).map(|_| MaybeUninit::uninit()).collect(),
            start: AtomicUsize::new(0),
            end: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }
    fn insert_value(&self, idx: usize, value: T) {
        unsafe {
            let buffer_ptr = self.buffer.as_ptr() as *mut MaybeUninit<T>;
            buffer_ptr.add(idx).drop_in_place();
            buffer_ptr.add(idx).write(MaybeUninit::new(value));
        }
    }
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&self, value: T) -> Result<(), String> {
        if self.len() == self.capacity() {
            return Err("Buffer is full".into());
        }
        let current_end = self.end.load(Ordering::Acquire);
        let new_end = if current_end + 1 < self.buffer.capacity() {
            current_end + 1
        } else {
            0
        };
        self.insert_value(current_end, value);
        self.count.fetch_add(1, Ordering::Relaxed);
        self.end.store(new_end, Ordering::Release);
        Ok(())
    }
    unsafe fn get_value(&self, idx: usize) -> T {
        unsafe {
            let buffer_ptr = self.buffer.as_ptr() as *mut MaybeUninit<T>;
            let value = std::ptr::replace(buffer_ptr.add(idx), MaybeUninit::uninit());
            value.assume_init()
        }
    }
    pub fn pop(&self) -> Option<T> {
        let current_start = self.start.load(Ordering::Acquire);
        let current_end = self.end.load(Ordering::Acquire);
        if current_start == current_end && self.count.load(Ordering::Relaxed) == 0 {
            return None;
        }

        let value = unsafe { self.get_value(current_start) };
        self.count.fetch_sub(1, Ordering::Relaxed);
        let new_start = if current_start + 1 >= self.buffer.capacity() {
            0
        } else {
            current_start + 1
        };
        self.start.store(new_start, Ordering::Release);
        Some(value)
    }
}

impl<T> Default for LockFreeBoundedRingBuffer<T> {
    fn default() -> Self {
        Self::new(Self::DEFAULT_BUFFER_SIZE)
    }
}
