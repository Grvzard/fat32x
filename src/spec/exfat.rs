use scroll::{self, Pread, LE};

#[derive(Debug, Pread)]
pub struct BootSec {
    pub jmp_boot: [u8; 3],     // `unused`
    file_system_name: [u8; 8], // check only, "EXFAT   "
    must_be_zero: [u8; 53],    // check only, zeros
    pub partition_offset: u64,
    pub volumn_length: u64,
    pub fat_offset: u32,
    pub fat_length: u32,
    pub cluster_heap_offset: u32,
    pub cluster_count: u32, // max: 0xFFFFFFF5
    pub first_cluster_of_root_dir: u32,
    pub volumn_serial_number: u32, // `unused`
    pub file_system_revision: [u8; 2], /* check only,
                                   upper byte for major and lower byte for minor */
    pub volumn_flags: u16,          // `unused`
    pub bytes_per_sector_shift: u8, // check, 9..=12 (512 to 4096 bytes)
    pub sectors_per_cluster_shift: u8, /* check, 0..=(25 - bytes_per_sector_shift)
                                    (1 sector to 32MB) */
    pub number_of_fats: u8,
    pub drive_select: u8,     // `unused`
    pub percent_in_use: u8,   // `unused`
    pub reserved: [u8; 7],    // `unused`
    pub boot_code: [u8; 390], // `unused`
    boot_signature: u16,      // check only, "0xAA55"
}

#[allow(dead_code)]
impl BootSec {
    pub fn new(buf: &[u8; 512]) -> Result<Self, scroll::Error> {
        buf.pread_with(0, LE)
    }

    pub fn is_valid(&self) -> bool {
        self.file_system_name == "EXFAT   ".as_bytes()
            && self.must_be_zero.iter().all(|&b| b == 0)
            && self.boot_signature == 0xAA55
            && self.file_system_revision[1] == 1 // The revision number of this spec is 1.0
            && self.number_of_fats == 1 // only support
            && (9..=12).contains(&self.bytes_per_sector_shift)
            && (0..=(25 - self.bytes_per_sector_shift))
                .contains(&self.sectors_per_cluster_shift)
    }

    pub fn bytes_per_sec(&self) -> u32 {
        1 << self.bytes_per_sector_shift
    }

    pub fn secs_per_clus(&self) -> u32 {
        (1 << self.sectors_per_cluster_shift) as u32
    }

    pub fn bytes_per_clus(&self) -> u32 {
        self.secs_per_clus() * self.bytes_per_sec()
    }
}

#[allow(dead_code)]
pub fn boot_checksum(bytes: &[u8], bytes_per_sec: u16) -> u32 {
    let num_of_bytes = (bytes_per_sec * 11) as usize;
    assert!(bytes.len() >= num_of_bytes);

    (0..num_of_bytes).fold(0, |sum, i| {
        if i == 106 || i == 107 || i == 112 {
            sum
        } else {
            (sum >> 1)
                .wrapping_add(bytes[i] as u32)
                .wrapping_add(if sum & 1 != 0 { 0x80000000 } else { 0 })
        }
    })
}
#[allow(dead_code)]
pub fn entset_checksum(bytes: &[u8], secondary_count: u8) -> u16 {
    let num_of_bytes = (secondary_count + 1) as usize * 32;
    assert!(bytes.len() >= num_of_bytes);

    (0..num_of_bytes).fold(0, |sum, i| {
        if i == 2 || i == 3 {
            sum
        } else {
            (sum >> 1)
                .wrapping_add(bytes[i] as u16)
                .wrapping_add(if sum & 1 != 0 { 0x8000 } else { 0 })
        }
    })
}

#[derive(Debug)]
pub struct DateTime {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl From<u32> for DateTime {
    fn from(val: u32) -> Self {
        let year = (val >> (9 + 16)) as u8;
        let month = (val >> (5 + 16) & 0xF) as u8;
        let day = (val >> 16 & 0x1F) as u8;
        let hour = (val >> 11 & 0x1F) as u8;
        let minute = (val >> 5 & 0x3F) as u8;
        let second = (val & 0x1F) as u8;
        DateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }
}

pub mod dirent {
    // use std::fmt;

    use std::time::{Duration, SystemTime};

    use chrono::{FixedOffset, TimeZone};
    use scroll::{Pread, LE};

    enum Type {
        AllocBitmap,
        UpcaseTable,
        VolumnLabel,
        FileOrDir,
        StreamExt,
        FileName,
        Unused,
        FinalUnused,
        // ...
    }

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("scroll read failed")]
        Scroll(#[from] scroll::Error),
        #[error("dir entries read failed")]
        ReadFailed,
        #[error("undefined entry type 0x{0:X}")]
        Undefined(u8),
    }

    impl TryFrom<u8> for Type {
        type Error = Error;
        fn try_from(value: u8) -> Result<Self, Self::Error> {
            match value {
                0x81 => Ok(Type::AllocBitmap),
                0x82 => Ok(Type::UpcaseTable),
                0x83 => Ok(Type::VolumnLabel),
                0x85 => Ok(Type::FileOrDir),
                0xC0 => Ok(Type::StreamExt),
                0xC1 => Ok(Type::FileName),
                0x01..=0x7F => Ok(Type::Unused),
                0 => Ok(Type::FinalUnused),
                typ => Err(Self::Error::Undefined(typ)),
            }
        }
    }

    impl Type {
        #[allow(dead_code)]
        #[inline]
        pub fn in_use(&self) -> bool {
            !matches!(*self, Self::Unused | Self::FinalUnused)
        }
    }

    #[derive(Debug, scroll::Pread)]
    pub struct AllocBitmap {
        pub bitmap_flags: u8,
        pub reserved: [u8; 18], // `unused`
        pub first_cluster: u32,
        pub data_length: u64,
    }
    #[derive(Debug, scroll::Pread)]
    pub struct UpcaseTable {
        pub reserved_1: [u8; 3], // `unused`
        pub table_checksum: u32,
        pub reserved_2: [u8; 12], // `unused`
        pub first_cluster: u32,
        pub data_length: u64,
    }
    #[derive(Debug, scroll::Pread)]
    pub struct VolumnLabel {
        pub chars_cnt: u8, // 0..=11
        pub volumn_label: [u16; 11],
        pub reserved: [u8; 8], // `unused`
    }
    #[derive(Debug, scroll::Pread)]
    pub struct FileOrDir {
        pub secondary_cnt: u8,
        pub set_checksum: u16,
        pub file_attributes: u16,
        pub reserved_1: [u8; 2], // `unused`
        pub create_dt: u32,      // upper 16 bits contains Date and lower 16 bits contains Time
        pub last_mod_dt: u32,
        pub last_acc_dt: u32,
        pub create_10ms_incr: u8,
        pub last_mod_10ms_incr: u8,
        pub create_tz_off: u8,
        pub last_mod_tz_off: u8,
        pub last_acc_tz_off: u8,
        pub reserved_2: [u8; 7], // `unused`
    }
    #[derive(Debug, scroll::Pread)]
    pub struct StreamExt {
        pub gen_secondary_flags: u8,
        pub reserved_1: [u8; 1], // `unused`
        pub name_length: u8,     // 1..=255
        pub name_hash: u16,
        pub reserved_2: [u8; 2], // `unused`
        pub valid_data_length: u64,
        pub reserved_3: [u8; 4], // `unused`
        pub first_cluster: u32,
        pub data_length: u64,
    }
    #[derive(Debug, scroll::Pread)]
    pub struct FileName {
        pub gen_secondary_flags: u8, // `unused`, zero
        pub filename: [u16; 15],
    }

    impl FileOrDir {
        pub fn is_rdonly(&self) -> bool {
            self.file_attributes & 0x01u16 != 0
        }
        pub fn is_hidden(&self) -> bool {
            self.file_attributes & 0x02u16 != 0
        }
        pub fn is_system(&self) -> bool {
            self.file_attributes & 0x04u16 != 0
        }
        pub fn is_dir(&self) -> bool {
            self.file_attributes & 0x10u16 != 0
        }
        #[allow(dead_code)]
        pub fn is_archive(&self) -> bool {
            self.file_attributes & 0x20u16 != 0
        }

        fn make_time(datetime: u32, tz_off: u8) -> Option<SystemTime> {
            let dt = super::DateTime::from(datetime);
            const QUARTER: i32 = 15 * 60;
            let tz = if tz_off & 0x80 != 0 {
                let val = (tz_off - 0x80) as i32;
                if val < 0x40 {
                    FixedOffset::east_opt(val * QUARTER)?
                } else {
                    FixedOffset::west_opt(((val ^ 0x7F) + 1) * QUARTER)?
                }
            } else {
                FixedOffset::east_opt(0)?
            };
            Some(
                tz.with_ymd_and_hms(
                    1980 + dt.year as i32,
                    dt.month.into(),
                    dt.day.into(),
                    dt.hour.into(),
                    dt.minute.into(),
                    dt.second.into(),
                )
                .single()?
                .into(),
            )
        }

        pub fn crt_time(&self) -> SystemTime {
            Self::make_time(self.create_dt, self.create_tz_off)
                .map(|time| time + Duration::new(0, self.create_10ms_incr as u32 * 10_000_000))
                .unwrap_or(SystemTime::UNIX_EPOCH)
        }
        pub fn mod_time(&self) -> SystemTime {
            Self::make_time(self.last_mod_dt, self.last_mod_tz_off)
                .map(|time| time + Duration::new(0, self.last_mod_10ms_incr as u32 * 10_000_000))
                .unwrap_or(SystemTime::UNIX_EPOCH)
        }
        pub fn acc_time(&self) -> SystemTime {
            Self::make_time(self.last_acc_dt, self.last_acc_tz_off)
                .unwrap_or(SystemTime::UNIX_EPOCH)
        }
    }

    impl From<&FileName> for String {
        fn from(ent: &FileName) -> Self {
            let mut term_idx = 15; // 15: ent.filename.len()
            for (i, &c) in ent.filename.iter().enumerate() {
                if c == 0x0000u16 {
                    term_idx = i;
                    break;
                }
            }
            String::from_utf16_lossy(&ent.filename[0..term_idx])
        }
    }

    pub enum EntrySet {
        // (u32, u32): the on-disk position (clus_no, offset in that cluster) of this entry
        FileOrDir(FileOrDir, (u32, u32)),
        StreamExt(StreamExt),
        FileName(FileName),
    }

    impl EntrySet {
        pub fn is_primary(&self) -> bool {
            matches!(*self, Self::FileOrDir(..))
        }
    }

    impl From<DirEnt> for Option<EntrySet> {
        fn from(ent: DirEnt) -> Self {
            match ent {
                DirEnt::FileOrDir(ent, pos) => Some(EntrySet::FileOrDir(ent, pos)),
                DirEnt::StreamExt(ent) => Some(EntrySet::StreamExt(ent)),
                DirEnt::FileName(ent) => Some(EntrySet::FileName(ent)),
                _ => None,
            }
        }
    }

    #[derive(Debug)]
    pub enum DirEnt {
        AllocBitmap(AllocBitmap),
        UpcaseTable(UpcaseTable),
        VolumnLabel(VolumnLabel),
        FileOrDir(FileOrDir, (u32, u32)),
        StreamExt(StreamExt),
        FileName(FileName),
        Unused,
        FinalUnused,
    }

    impl DirEnt {
        pub const SZ: usize = 32;
        pub fn new(buf: &[u8], clusno: u32, offset: u32) -> Result<Self, Error> {
            if buf.len() < 32 {
                return Err(Error::ReadFailed);
            }
            let entry_type_byte: u8 = buf.pread_with(0, LE)?;
            let entry_type: Type = entry_type_byte.try_into()?;
            let rest = &buf[1..];
            match entry_type {
                Type::AllocBitmap => Ok(Self::AllocBitmap(rest.pread_with(0, LE)?)),
                Type::UpcaseTable => Ok(Self::UpcaseTable(rest.pread_with(0, LE)?)),
                Type::VolumnLabel => Ok(Self::VolumnLabel(rest.pread_with(0, LE)?)),
                Type::FileOrDir => Ok(Self::FileOrDir(rest.pread_with(0, LE)?, (clusno, offset))),
                Type::StreamExt => Ok(Self::StreamExt(rest.pread_with(0, LE)?)),
                Type::FileName => Ok(Self::FileName(rest.pread_with(0, LE)?)),
                Type::Unused => Ok(DirEnt::Unused),
                Type::FinalUnused => Ok(DirEnt::FinalUnused),
            }
        }
    }

    // TODO
    // impl fmt::Display for DirEnt {
    //     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    //         write!(f, "{:?}", self)
    //     }
    // }
}

#[derive(Debug)]
pub enum FatEnt {
    // Free,
    Chain(u32),
    BadCluster,
    EndOfChain,
    Reserved,
}
impl FatEnt {
    pub const SZ: usize = 4;
}
