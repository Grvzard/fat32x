// References:
// [1] https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("dir entries reduction failed")]
    DirEntReductionFailed,
    #[error("dir entries read failed")]
    DirEntReadFailed,
    #[error("scroll read failed")]
    Scroll(#[from] scroll::Error),
    #[error("undefined entry type 0x{0:X}")]
    UndefinedDirEntry(u8),
}

pub mod spec {
    use scroll::{self, Pread, LE};

    #[derive(Debug)]
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
            Ok(BootSec {
                jmp_boot: buf.pread_with(0, LE)?,
                file_system_name: buf.pread_with(3, LE)?,
                must_be_zero: buf.pread_with(11, LE)?,
                partition_offset: buf.pread_with(64, LE)?,
                volumn_length: buf.pread_with(72, LE)?,
                fat_offset: buf.pread_with(80, LE)?,
                fat_length: buf.pread_with(84, LE)?,
                cluster_heap_offset: buf.pread_with(88, LE)?,
                cluster_count: buf.pread_with(92, LE)?,
                first_cluster_of_root_dir: buf.pread_with(96, LE)?,
                volumn_serial_number: buf.pread_with(100, LE)?,
                file_system_revision: buf.pread_with(104, LE)?,
                volumn_flags: buf.pread_with(106, LE)?,
                bytes_per_sector_shift: buf.pread_with(108, LE)?,
                sectors_per_cluster_shift: buf.pread_with(109, LE)?,
                number_of_fats: buf.pread_with(110, LE)?,
                drive_select: buf.pread_with(111, LE)?,
                percent_in_use: buf.pread_with(112, LE)?,
                reserved: buf.pread_with(113, LE)?,
                boot_code: buf.pread_with(120, LE)?,
                boot_signature: buf.pread_with(510, LE)?,
            })
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

        impl TryFrom<u8> for Type {
            type Error = crate::exfat::Error;
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
                    typ => Err(Self::Error::UndefinedDirEntry(typ)),
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

        #[derive(Debug)]
        pub struct AllocBitmap {
            pub bitmap_flags: u8,
            pub reserved: [u8; 18], // `unused`
            pub first_cluster: u32,
            pub data_length: u64,
        }
        #[derive(Debug)]
        pub struct UpcaseTable {
            pub reserved_1: [u8; 3], // `unused`
            pub table_checksum: u32,
            pub reserved_2: [u8; 12], // `unused`
            pub first_cluster: u32,
            pub data_length: u64,
        }
        #[derive(Debug)]
        pub struct VolumnLabel {
            pub chars_cnt: u8, // 0..=11
            pub volumn_label: [u16; 11],
            pub reserved: [u8; 8], // `unused`
        }
        #[derive(Debug)]
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

            // the on-disk position (clus_no and offset in that cluster) of this entry
            pub ent_clusno: u32,
            pub ent_off: u32,
        }
        #[derive(Debug)]
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
        #[derive(Debug)]
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
                    .map(|time| {
                        time + Duration::new(0, self.last_mod_10ms_incr as u32 * 10_000_000)
                    })
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
            FileOrDir(FileOrDir),
            StreamExt(StreamExt),
            FileName(FileName),
        }

        impl EntrySet {
            pub fn is_primary(&self) -> bool {
                matches!(*self, Self::FileOrDir(_))
            }
        }

        impl From<DirEnt> for Option<EntrySet> {
            fn from(ent: DirEnt) -> Self {
                match ent {
                    DirEnt::FileOrDir(ent) => Some(EntrySet::FileOrDir(ent)),
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
            FileOrDir(FileOrDir),
            StreamExt(StreamExt),
            FileName(FileName),
            Unused,
            FinalUnused,
        }

        use crate::exfat::Error;
        impl DirEnt {
            pub const SZ: usize = 32;
            pub fn new(buf: &[u8], clusno: u32, offset: u32) -> Result<Self, Error> {
                if buf.len() < 32 {
                    return Err(Error::DirEntReadFailed);
                }
                let entry_type_byte: u8 = buf.pread_with(0, LE)?;
                let entry_type: Type = entry_type_byte.try_into()?;
                match entry_type {
                    Type::AllocBitmap => Ok(Self::AllocBitmap(AllocBitmap {
                        bitmap_flags: buf.pread_with(1, LE)?,
                        reserved: buf.pread_with(2, LE)?,
                        first_cluster: buf.pread_with(20, LE)?,
                        data_length: buf.pread_with(24, LE)?,
                    })),
                    Type::UpcaseTable => Ok(Self::UpcaseTable(UpcaseTable {
                        reserved_1: buf.pread_with(1, LE)?,
                        table_checksum: buf.pread_with(4, LE)?,
                        reserved_2: buf.pread_with(8, LE)?,
                        first_cluster: buf.pread_with(20, LE)?,
                        data_length: buf.pread_with(24, LE)?,
                    })),
                    Type::VolumnLabel => Ok(Self::VolumnLabel(VolumnLabel {
                        chars_cnt: buf.pread_with(1, LE)?,
                        volumn_label: buf.pread_with(2, LE)?,
                        reserved: buf.pread_with(24, LE)?,
                    })),
                    Type::FileOrDir => Ok(Self::FileOrDir(FileOrDir {
                        secondary_cnt: buf.pread_with(1, LE)?,
                        set_checksum: buf.pread_with(2, LE)?,
                        file_attributes: buf.pread_with(4, LE)?,
                        reserved_1: buf.pread_with(6, LE)?,
                        create_dt: buf.pread_with(8, LE)?,
                        last_mod_dt: buf.pread_with(12, LE)?,
                        last_acc_dt: buf.pread_with(16, LE)?,
                        create_10ms_incr: buf.pread_with(20, LE)?,
                        last_mod_10ms_incr: buf.pread_with(21, LE)?,
                        create_tz_off: buf.pread_with(22, LE)?,
                        last_mod_tz_off: buf.pread_with(23, LE)?,
                        last_acc_tz_off: buf.pread_with(24, LE)?,
                        reserved_2: buf.pread_with(25, LE)?,
                        ent_clusno: clusno,
                        ent_off: offset,
                    })),
                    Type::StreamExt => Ok(Self::StreamExt(StreamExt {
                        gen_secondary_flags: buf.pread_with(1, LE)?,
                        reserved_1: buf.pread_with(2, LE)?,
                        name_length: buf.pread_with(3, LE)?,
                        name_hash: buf.pread_with(4, LE)?,
                        reserved_2: buf.pread_with(6, LE)?,
                        valid_data_length: buf.pread_with(8, LE)?,
                        reserved_3: buf.pread_with(16, LE)?,
                        first_cluster: buf.pread_with(20, LE)?,
                        data_length: buf.pread_with(24, LE)?,
                    })),
                    Type::FileName => Ok(Self::FileName(FileName {
                        gen_secondary_flags: buf.pread_with(1, LE)?,
                        filename: buf.pread_with(2, LE)?,
                    })),
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
}

use std::io::{Read, Seek, SeekFrom};

use scroll::{Pread, LE};

use crate::fio::{self, Finfo};
use spec::{
    dirent::{DirEnt, EntrySet},
    BootSec, FatEnt,
};

const SEC_SZ: usize = 512;
type Sec = [u8; SEC_SZ];

#[allow(dead_code)]
pub struct Fio<D: Seek + Read> {
    device: D,
    root_clusno: u32,
    bitmap_clusno: u32,
    sec_sz: u32,
    secs_per_clus: u32,
    clus_heap_offset: u32, // in sectors
    clus_heap_base: u64,   // in bytes
    clus_sz: u32,
    clus_cnt: u32,
    fat_offset: u32, // in sectors
    dirents_per_sec: u32,
    pub bootsec: BootSec,
}

#[allow(dead_code)]
impl<D: Seek + Read> Fio<D> {
    pub fn new(mut device: D) -> Self {
        let mut buf: Sec = [0u8; SEC_SZ];
        device.seek(SeekFrom::Start(0)).unwrap();
        device.read_exact(&mut buf).unwrap();

        let bootsec = BootSec::new(&buf).unwrap();
        assert!(bootsec.is_valid());
        let mut fio = Fio {
            device,
            root_clusno: bootsec.first_cluster_of_root_dir,
            bitmap_clusno: 0,
            sec_sz: bootsec.bytes_per_sec(),
            secs_per_clus: bootsec.secs_per_clus(),
            clus_heap_offset: bootsec.cluster_heap_offset,
            clus_heap_base: bootsec.cluster_heap_offset as u64 * bootsec.bytes_per_sec() as u64,
            clus_sz: bootsec.bytes_per_clus(),
            clus_cnt: bootsec.cluster_count,
            fat_offset: bootsec.fat_offset,
            dirents_per_sec: bootsec.bytes_per_sec() / 32,
            bootsec,
        };

        let root_ents = fio.read_dirents(fio.root_clusno);
        if let Some(DirEnt::AllocBitmap(allocmap)) = root_ents
            .into_iter()
            .find(|ent| matches!(ent, DirEnt::AllocBitmap(_)))
        {
            fio.bitmap_clusno = allocmap.first_cluster;
        } else {
            panic!("[fio] init: allocation map not found in root dir");
        }
        fio
    }

    pub fn read_clus(&mut self, clusno: u32) -> Vec<u8> {
        if clusno < 2 || clusno > self.clus_cnt + 1 {
            println!("[fio] read_clus: cluster over reading");
            return vec![];
        }
        let mut buf = vec![0u8; self.clus_sz as usize];
        self.device
            .seek(SeekFrom::Start(
                self.clus_heap_base + (clusno - 2) as u64 * self.clus_sz as u64,
            ))
            .unwrap();
        self.device.read_exact(&mut buf).unwrap();
        buf
    }

    pub fn read_sec(&mut self, secno: u64) -> Vec<u8> {
        let mut buf = vec![0u8; self.sec_sz as usize];
        self.device
            .seek(SeekFrom::Start(secno * self.sec_sz as u64))
            .unwrap();
        self.device.read_exact(&mut buf).unwrap();
        buf
    }

    fn read_fat(&mut self, clusno: u32) -> FatEnt {
        if clusno < 2 || clusno > self.clus_cnt + 1 {
            println!("[fio] read_fat: FAT over reading");
            return FatEnt::Reserved;
        }
        // TODO: check out the bitmap first
        // if !self.read_allocbit(clusno) {
        //     return FatEnt::Free;
        // }
        let sec_no = clusno / self.dirents_per_sec;
        let ent_off = (clusno % self.dirents_per_sec) as usize;
        let sec = self.read_sec((self.fat_offset + sec_no).into());
        let off = FatEnt::SZ * ent_off;
        let ent: u32 = sec.pread_with(off, LE).unwrap();

        println!("{}", ent);
        if ent <= self.clus_cnt + 1 {
            if ent >= 2 {
                FatEnt::Chain(ent)
            } else {
                FatEnt::Reserved
            }
        } else if ent == 0xFFFFFFFF {
            FatEnt::EndOfChain
        } else if ent == 0xFFFFFFF7 {
            FatEnt::BadCluster
        } else {
            FatEnt::Reserved
        }
    }

    // TODO
    // fn read_allocbit(&mut self, clusno: u32) -> bool {}

    // walking the fat chain, return cluster numbers including the first one
    fn walk_fats(&mut self, mut clusno: u32) -> Vec<u32> {
        let mut ret = vec![];
        loop {
            ret.push(clusno);
            match self.read_fat(clusno) {
                // FatEnt::Free => panic!("[fio] walk_fats: unexpected fat entry"),
                FatEnt::Chain(next) => clusno = next,
                FatEnt::BadCluster => panic!("[fio] walk_fats: read a bad clustor"),
                FatEnt::EndOfChain => break,
                FatEnt::Reserved => {
                    // TODO: after complete read_allocbit
                    break;
                    // panic!("[fio] walk_fats: reserved fat entry (clusno:{})", clusno)
                }
            }
        }
        ret
    }

    // given a cluster number, return the absolute sector numbers this cluster holds
    fn secnos_of_clusno(&self, mut clusno: u32) -> impl Iterator<Item = u64> {
        clusno -= 2;
        let off = self.clus_heap_offset as u64 + clusno as u64 * self.secs_per_clus as u64;
        off..off + self.secs_per_clus as u64
    }

    pub fn read_dirents(&mut self, clusno: u32) -> Vec<DirEnt> {
        let mut ret = vec![];

        let clusno_list = self.walk_fats(clusno);
        'reading: for clusno in clusno_list.into_iter() {
            let mut off = 0;
            for secno in self.secnos_of_clusno(clusno) {
                let sec = self.read_sec(secno);
                for buf in sec.chunks(DirEnt::SZ) {
                    match DirEnt::new(buf, clusno, off) {
                        Ok(dirent) => match dirent {
                            DirEnt::Unused => (),
                            DirEnt::FinalUnused => break 'reading,
                            _ => ret.push(dirent),
                        },
                        Err(err) => panic!("[fio] read_dirents: {}", err),
                    }
                    off += 1;
                }
            }
        }
        ret
    }
}

impl TryFrom<Vec<EntrySet>> for fio::Finfo {
    type Error = Error;
    fn try_from(ents: Vec<EntrySet>) -> Result<Self, Self::Error> {
        if ents.len() < 3 {
            return Err(Self::Error::DirEntReductionFailed);
        }

        let (ent_file, ent_stream) = match (&ents[0], &ents[1]) {
            (EntrySet::FileOrDir(ent0), EntrySet::StreamExt(ent1)) => (ent0, ent1),
            _ => return Err(Self::Error::DirEntReductionFailed),
        };

        let mut name = String::new();
        for ent in ents[2..].iter() {
            let ent_name = match ent {
                EntrySet::FileName(ent_name) => ent_name,
                _ => return Err(Self::Error::DirEntReductionFailed),
            };
            name.push_str(&String::from(ent_name));
        }

        // TODO
        // 'check: {}

        Ok(Finfo {
            id: (ent_file.ent_off as u64) << 32 | ent_file.ent_clusno as u64,
            name,
            acc_time: ent_file.acc_time(),
            crt_time: ent_file.crt_time(),
            wrt_time: ent_file.mod_time(),
            fst_clus: ent_stream.first_cluster,
            is_dir: ent_file.is_dir(),
            is_hidden: ent_file.is_hidden(),
            is_rdonly: ent_file.is_rdonly(),
            is_system: ent_file.is_system(),
            size32: 0,
            size: ent_stream.valid_data_length,
        })
    }
}

impl<D: Seek + Read> fio::Fio for Fio<D> {
    fn list_dir(&mut self, clusno: u32) -> Vec<fio::Finfo> {
        let mut ret = vec![];
        let ents = self.read_dirents(clusno);
        let mut pending_list = vec![];

        for ent in ents.into_iter() {
            if let Some(set_ent) = Option::<EntrySet>::from(ent) {
                if set_ent.is_primary() && !pending_list.is_empty() {
                    if let Ok(fi) = fio::Finfo::try_from(pending_list) {
                        ret.push(fi);
                    } else {
                        println!("[fio] list_dir: dirents reduction failed");
                    };
                    pending_list = vec![];
                }
                pending_list.push(set_ent);
            }
        }

        ret
    }

    fn list_root(&mut self) -> Vec<fio::Finfo> {
        self.list_dir(self.root_clusno)
    }

    fn read_file(&mut self, fi: &fio::Finfo, offset: u32, size: u32) -> Vec<u8> {
        let _ = fi;
        let _ = offset;
        let _ = size;
        vec![]
    }
}
