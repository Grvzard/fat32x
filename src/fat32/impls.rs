use std::{fs::File, os::unix::fs::FileExt};

use super::fio::Device;

pub struct BlkDevice {
    file: File,
}

impl BlkDevice {
    pub fn new(name: &str) -> Self {
        let file = File::options()
            .create(false)
            .write(false)
            .truncate(false)
            .read(true)
            .open(&name)
            .expect("device can't be opened");
        BlkDevice { file }
    }
}

impl Device for BlkDevice {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) {
        self.file.read_exact_at(buf, offset).unwrap()
    }
}
