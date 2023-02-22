use heapless::mpmc::Q2;

pub(super) struct Signal {
    inner: Q2<()>,
}

unsafe impl Sync for Signal {}

/// This could be better but at least it ain't the last thing (that was bad)
impl Signal {
    pub(super) const fn new() -> Self {
        Self { inner: Q2::new() }
    }

    #[must_use]
    pub(super) fn take_signal(&self) -> bool {
        self.inner.dequeue().is_some()
    }

    pub(super) fn clear(&self) {
        self.inner.dequeue();
    }

    pub(super) fn set_signal(&self) {
        self.inner.enqueue(()).ok();
    }
}
