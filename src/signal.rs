pub struct Signal {
    inner: core::cell::RefCell<bool>,
}

/// I recognizes that this is bad, not good, and very uncool. I don't have time
/// to decipher the gross C code I'm attempting to port. As such, this hack
/// will remain until I have the time, patiences, and courage to do it better.
/// (So this is here to stay basically because I doubt I will rewrite.)
impl Signal {
    pub const fn new() -> Self {
        Self {
            inner: core::cell::RefCell::new(false),
        }
    }

    pub fn check(&self) -> bool {
        unsafe { core::ptr::read_volatile(self.inner.as_ptr()) }
    }

    pub fn assign(&self, val: bool) {
        unsafe { core::ptr::write_volatile(self.inner.as_ptr(), val) };
    }

    pub fn set(&self) {
        self.assign(true);
    }

    pub fn reset(&self) {
        self.assign(false);
    }
}
