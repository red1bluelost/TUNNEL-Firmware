use crate::mem::{self, BufBox, VecBuf};

#[derive(Debug)]
pub struct Frame {
    pub stx: u8,
    pub length: u8,
    pub command: u8,
    pub data: BufBox,
    pub checksum: u16,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            stx: Default::default(),
            length: Default::default(),
            command: Default::default(),
            data: mem::alloc().unwrap(),
            checksum: Default::default(),
        }
    }
}

impl Clone for Frame {
    fn clone(&self) -> Self {
        let data =
            mem::alloc_init(VecBuf::from_slice(&self.data).unwrap()).unwrap();
        Self { data, ..*self }
    }
}

impl Frame {
    pub fn new(stx: u8, length: u8, command: u8, data: BufBox) -> Self {
        assert!(data.len() == length as _);
        let checksum = Self::calc_checksum(command, length, &data);
        Self {
            stx,
            length,
            command,
            data,
            checksum,
        }
    }

    pub fn calc_checksum(command: u8, length: u8, data: &[u8]) -> u16 {
        assert_eq!(length as usize, data.len());
        data.iter().fold(
            u16::wrapping_add(command.into(), length.into()),
            |acc, &val| u16::wrapping_add(acc, val.into()),
        )
    }

    pub fn checksum(&self) -> u16 {
        Self::calc_checksum(self.command, self.length, &self.data[..])
    }

    pub fn clear(&mut self) {
        *self = Default::default();
    }
}
