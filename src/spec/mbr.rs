#![allow(dead_code)]

use scroll::{Pread, LE};

#[derive(Debug, Pread)]
pub struct PartitionEntry {
    active: u8,
    first_sec: [u8; 3],
    typ: u8,
    last_sec: [u8; 3],
    lba: u32,
    nsecs: u32,
}

#[derive(Debug, Pread)]
pub struct Mbr {
    boot_code: [u8; 446],
    partition_1: PartitionEntry,
    partition_2: PartitionEntry,
    partition_3: PartitionEntry,
    partition_4: PartitionEntry,
    boot_sig: u16,
}

impl Mbr {
    pub fn new(buf: &[u8]) -> Result<Self, scroll::Error> {
        buf.pread_with(0, LE)
    }
}
