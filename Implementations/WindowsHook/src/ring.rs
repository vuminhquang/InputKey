use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Single-producer/single-consumer bounded queue. Producer never waits.
pub struct SpscRing<T: Copy, const N: usize> {
    head: AtomicUsize,
    tail: AtomicUsize,
    slots: [UnsafeCell<Option<T>>; N],
}
// Safety: only the producer writes a slot before publishing head; only the
// consumer reads it after observing head and clears it before publishing tail.
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
        let h = self.head.load(Ordering::Relaxed);
        if h.wrapping_sub(self.tail.load(Ordering::Acquire)) >= N {
            return false;
        }
        // SAFETY: this is the unique producer's unpublished slot.
        unsafe {
            *self.slots[h % N].get() = Some(value);
        }
        self.head.store(h.wrapping_add(1), Ordering::Release);
        true
    }
    pub fn pop(&self) -> Option<T> {
        let t = self.tail.load(Ordering::Relaxed);
        if t == self.head.load(Ordering::Acquire) {
            return None;
        }
        // SAFETY: the producer published this slot and cannot reuse it until tail advances.
        let value = unsafe { (*self.slots[t % N].get()).take() };
        self.tail.store(t.wrapping_add(1), Ordering::Release);
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
    pub down: bool,
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
    Literalize,
    FinalizeAndReplay,
    ResetAndReplayKeepDisplayed,
    RecordShortcut,
    ResetOnly,
}
