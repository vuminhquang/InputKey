use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct SpscRing<T: Copy, const N: usize> {
    head: AtomicUsize,
    tail: AtomicUsize,
    slots: [UnsafeCell<Option<T>>; N],
}

unsafe impl<T: Copy + Send, const N: usize> Sync for SpscRing<T, N> {}
unsafe impl<T: Copy + Send, const N: usize> Send for SpscRing<T, N> {}

impl<T: Copy, const N: usize> SpscRing<T, N> {
    pub fn new() -> Self {
        assert!(N > 0);
        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            slots: std::array::from_fn(|_| UnsafeCell::new(None)),
        }
    }

    pub fn push(&self, value: T) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        if head.wrapping_sub(self.tail.load(Ordering::Acquire)) >= N {
            return false;
        }
        unsafe {
            *self.slots[head % N].get() = Some(value);
        }
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) {
            return None;
        }
        let value = unsafe { (*self.slots[tail % N].get()).take() };
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        value
    }
}

impl<T: Copy, const N: usize> Default for SpscRing<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub kind: EventKind,
    pub vk: u16,
    pub modifiers: u8,
    pub target: usize,
    pub epoch: u64,
    pub scan_code: u16,
    pub extended: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventKind {
    #[default]
    Pass,
    TypeChar,
    Backspace,
    Escape,
    RawBoundary,
    FinalizeWithDelimiter,
    NaturalBoundary,
    MouseBoundary,
    ResetOnly,
}
