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

#[derive(Debug)]
pub struct DateField<const OFF: usize> {
    pub year: u8,
    pub month: u8,
    pub day: u8,
}

impl<const OFF: usize> Field for DateField<OFF> {
    fn load(buf: &[u8]) -> Self {
        let val = u16::from_le_bytes([buf[OFF], buf[OFF + 1]]);
        let year = (val >> 9) as u8;
        let month = (val >> 5 & 0xF) as u8;
        let day = (val & 0x1F) as u8;
        DateField { year, month, day }
    }
    fn dump(&self, buf: &mut [u8]) {
        let val =
            (self.year as u16) << 9 | ((self.month & 0xF) as u16) << 5 | (self.day & 0x1F) as u16;
        buf[OFF] = val as u8;
        buf[OFF + 1] = (val >> 8) as u8;
    }
}

#[derive(Debug)]
pub struct TimeField<const OFF: usize> {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl<const OFF: usize> Field for TimeField<OFF> {
    fn load(buf: &[u8]) -> Self {
        let val = u16::from_le_bytes([buf[OFF], buf[OFF + 1]]);
        let hour = (val >> 11) as u8;
        let minute = (val >> 5 & 0x3F) as u8;
        let second = (val & 0x1F) as u8;
        TimeField {
            hour,
            minute,
            second,
        }
    }
    fn dump(&self, buf: &mut [u8]) {
        let val = (self.hour as u16) << 11
            | ((self.minute & 0x3F) as u16) << 5
            | (self.second & 0x1F) as u16;
        buf[OFF] = val as u8;
        buf[OFF + 1] = (val >> 8) as u8;
    }
}
