use heapless::spsc::Consumer;

pub trait Exchange {
    fn exchange(&mut self, val: Self) -> Self;
}

impl<T> Exchange for T {
    fn exchange(&mut self, mut val: T) -> T {
        core::mem::swap(self, &mut val);
        val
    }
}

pub trait Zero {
    const ZERO: Self;
}

macro_rules! zero_impl {
    ($t:ty) => {
        impl Zero for $t {
            const ZERO: $t = 0;
        }
    };
}

zero_impl!(u8);
zero_impl!(u16);
zero_impl!(u32);
zero_impl!(u64);
zero_impl!(i8);
zero_impl!(i16);
zero_impl!(i32);
zero_impl!(i64);
zero_impl!(usize);
zero_impl!(isize);

pub const fn zeros<T: Zero + Copy, const N: usize>() -> [T; N] {
    [T::ZERO; N]
}

pub struct NullQueueConsumer<'a, T, const N: usize> {
    consumer: Consumer<'a, T, N>,
}

impl<'a, T, const N: usize> NullQueueConsumer<'a, T, N> {
    pub fn new(consumer: Consumer<'a, T, N>) -> Self {
        Self { consumer }
    }

    pub fn poll(&mut self) {
        self.consumer.dequeue();
    }
}
