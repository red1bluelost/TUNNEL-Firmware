use heapless::{
    pool,
    pool::singleton::{Box, Pool},
    Vec,
};

pub type VecBuf = Vec<u8, 255>;
pub type BufBox = Box<POOL>;

pool!(POOL: VecBuf);

#[inline(always)]
pub fn grow(memory: &'static mut [u8]) -> usize {
    POOL::grow(memory)
}

#[inline(always)]
pub fn alloc_init(buf: VecBuf) -> Option<BufBox> {
    POOL::alloc().map(|b| b.init(buf))
}

#[inline(always)]
pub fn alloc() -> Option<BufBox> {
    alloc_init(VecBuf::new())
}
