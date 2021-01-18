use std::{mem, ops, ptr, sync::atomic};

// Holds a string and zeros it when we're done.
pub struct ZeroOnDrop {
    inner: Inner,
}

impl ZeroOnDrop {
    pub fn new() -> Self {
        ZeroOnDrop {
            inner: Inner(String::new()),
        }
    }

    pub fn into_inner(mut self) -> String {
        mem::replace(&mut self.inner.0, String::new())
    }
}

impl ops::Deref for ZeroOnDrop {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.inner.0
    }
}

impl ops::DerefMut for ZeroOnDrop {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.0
    }
}

struct Inner(String);

impl Drop for Inner {
    fn drop(&mut self) {
        self.zero_memory();
    }
}

impl Inner {
    /// Sets all bytes of a String to 0
    fn zero_memory(&mut self) {
        let default = u8::default();

        for c in unsafe { self.0.as_bytes_mut() } {
            unsafe { ptr::write_volatile(c, default) };
        }

        atomic::fence(atomic::Ordering::SeqCst);
        atomic::compiler_fence(atomic::Ordering::SeqCst);
    }
}
