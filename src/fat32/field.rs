pub trait Field {
    fn load(buf: &[u8]) -> Self;
    fn dump(&self, buf: &mut [u8]);
}

#[derive(Debug)]
pub struct U8Field<const OFF: usize> {
    pub value: u8,
}

impl<const OFF: usize> Field for U8Field<OFF> {
    fn load(buf: &[u8]) -> Self {
        let value = buf[OFF];
        U8Field::<OFF> { value }
    }
    fn dump(&self, buf: &mut [u8]) {
        buf[OFF] = self.value
    }
}

#[derive(Debug)]
pub struct U16Field<const OFF: usize> {
    pub value: u16,
}

impl<const OFF: usize> Field for U16Field<OFF> {
    fn load(buf: &[u8]) -> Self {
        let value = (buf[OFF + 1] as u16) << 8 | (buf[OFF] as u16);
        U16Field::<OFF> { value }
    }
    fn dump(&self, buf: &mut [u8]) {
        buf[OFF] = self.value as u8;
        buf[OFF + 1] = (self.value >> 8) as u8;
    }
}

#[derive(Debug)]
pub struct U32Field<const OFF: usize> {
    pub value: u32,
}

impl<const OFF: usize> Field for U32Field<OFF> {
    fn load(buf: &[u8]) -> Self {
        U32Field::<OFF> {
            value: u32::from_le_bytes([buf[OFF], buf[OFF + 1], buf[OFF + 2], buf[OFF + 3]]),
        }
    }
    fn dump(&self, buf: &mut [u8]) {
        let bytes = self.value.to_le_bytes();
        buf[OFF] = bytes[0];
        buf[OFF + 1] = bytes[1];
        buf[OFF + 2] = bytes[2];
        buf[OFF + 3] = bytes[3];
    }
}

#[derive(Debug)]
pub struct BytesField<const OFF: usize, const LEN: usize> {
    pub value: [u8; LEN],
}

impl<const OFF: usize, const LEN: usize> Field for BytesField<OFF, LEN> {
    fn load(buf: &[u8]) -> Self {
        let mut value = [0u8; LEN];
        for i in 0..LEN {
            value[i] = buf[OFF + i];
        }
        BytesField::<OFF, LEN> { value }
    }
    fn dump(&self, buf: &mut [u8]) {
        for i in 0..LEN {
            buf[OFF + i] = self.value[i];
        }
    }
}

#[derive(Debug)]
pub struct Utf16Field<const OFF: usize, const LEN: usize> {
    pub value: [u16; LEN],
}

impl<const OFF: usize, const LEN: usize> Field for Utf16Field<OFF, LEN> {
    fn load(buf: &[u8]) -> Self {
        let mut value = [0u16; LEN];
        for i in 0..LEN {
            value[i] = u16::from_le_bytes([buf[OFF + i * 2], buf[OFF + i * 2 + 1]]);
        }
        Utf16Field::<OFF, LEN> { value }
    }
    fn dump(&self, buf: &mut [u8]) {
        for i in 0..LEN {
            buf[OFF + i * 2] = self.value[i] as u8;
            buf[OFF + i * 2 + 1] = (self.value[i] >> 8) as u8;
        }
    }
}
