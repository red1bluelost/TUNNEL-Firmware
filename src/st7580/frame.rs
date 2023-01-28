#[repr(C)]
#[derive(Debug, Clone)]
pub struct Frame {
    pub stx: u8,
    pub length: u8,
    pub command: u8,
    pub data: [u8; 255],
    pub checksum: u16,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            stx: Default::default(),
            length: Default::default(),
            command: Default::default(),
            data: [0; 255],
            checksum: Default::default(),
        }
    }
}

impl Frame {
    fn new(stx: u8, length: u8, command: u8, data: [u8; 255]) -> Self {
        Self {
            stx,
            length,
            command,
            data,
            checksum: Self::calc_checksum(
                command,
                length,
                &data[..length as usize],
            ),
        }
    }

    fn calc_checksum(command: u8, length: u8, data: &[u8]) -> u16 {
        assert_eq!(length as usize, data.len());
        data.iter().fold(
            u16::wrapping_add(command.into(), length.into()),
            |acc, &val| u16::wrapping_add(acc, val.into()),
        )
    }

    fn checksum(&self) -> u16 {
        Self::calc_checksum(
            self.command,
            self.length,
            &self.data[..self.length as _],
        )
    }

    fn clear(&mut self) {
        *self = Default::default();
    }
}
