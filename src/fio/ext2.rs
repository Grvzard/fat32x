// References:
// [1] https://www.nongnu.org/ext2-doc/ext2.html

#![allow(dead_code)]

use std::io::{Read, Seek, SeekFrom};

use crate::spec::ext2::Sblk;

pub struct Fio<D: Seek + Read> {
    blk_sz: u32,
    bgp_per_block: u32,
    device: D,
    pub sblk: Sblk,
}

impl<D: Seek + Read> Fio<D> {
    pub fn new(mut device: D) -> Self {
        let mut buf = [0u8; 1024];

        device.seek(SeekFrom::Start(1024)).unwrap();
        device.read_exact(&mut buf).unwrap();
        let sblk = Sblk::new(&buf).unwrap();
        assert!(sblk.is_valid());

        Fio {
            blk_sz: sblk.blk_sz(),
            bgp_per_block: sblk.blk_sz() / 32,
            device,
            sblk,
        }
    }

    fn read_block(&mut self, blk_no: u32) -> Vec<u8> {
        let mut buf = vec![0u8; self.blk_sz as usize];
        self.device
            .seek(SeekFrom::Start((blk_no * self.blk_sz) as u64))
            .unwrap();
        self.device.read_exact(&mut buf).unwrap();
        buf
    }
}
