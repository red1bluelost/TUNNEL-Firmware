pub trait Exchange {
    fn exchange(&mut self, val: Self) -> Self;
}

impl<T> Exchange for T {
    fn exchange(&mut self, mut val: T) -> T {
        core::mem::swap(self, &mut val);
        val
    }
}
