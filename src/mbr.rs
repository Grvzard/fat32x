#![allow(dead_code)]

use scroll::{ctx::TryFromCtx, Pread, LE};

#[derive(Debug)]
pub struct PartitionEntry {
    active: u8,
    first_sec: [u8; 3],
    typ: u8,
    last_sec: [u8; 3],
    lba: u32,
    nsecs: u32,
}

impl<'a> TryFromCtx<'a, scroll::Endian> for PartitionEntry {
    type Error = scroll::Error;
    fn try_from_ctx(from: &'a [u8], _ctx: scroll::Endian) -> Result<(Self, usize), Self::Error> {
        Ok((
            PartitionEntry {
                active: from.pread_with(0, LE)?,
                first_sec: from.pread_with(1, LE)?,
                typ: from.pread_with(4, LE)?,
                last_sec: from.pread_with(5, LE)?,
                lba: from.pread_with(8, LE)?,
                nsecs: from.pread_with(12, LE)?,
            },
            16,
        ))
    }
}

#[derive(Debug)]
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
        Ok(Mbr {
            boot_code: buf.pread_with(0, LE)?,
            partition_1: buf.pread_with(446, LE)?,
            partition_2: buf.pread_with(462, LE)?,
            partition_3: buf.pread_with(478, LE)?,
            partition_4: buf.pread_with(494, LE)?,
            boot_sig: buf.pread_with(510, LE)?,
        })
    }
}
