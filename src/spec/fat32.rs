// References:
// [1] https://download.microsoft.com/download/1/6/1/161ba512-40e2-4cc9-843a-923143f3456c/fatgen103.doc
// [2] http://elm-chan.org/docs/fat_e.html
// [3] https://en.wikipedia.org/wiki/Design_of_the_FAT_file_system#FAT

use std::time::SystemTime;

use chrono::{Local, TimeZone};
use scroll::{self, Pread, LE};

pub type ClusNo = u32; // static

#[derive(Debug)]
pub struct BootSec {
    // > 0-35
    // BS_JmpBoot
    pub bs_oem_name: [u8; 8], // `unused`
    pub bpb_byts_per_sec: u16,
    pub bpb_sec_per_clus: u8,
    pub bpb_rsvd_sec_cnt: u16,
    pub bpb_num_fats: u8,
    pub bpb_root_ent_cnt: u16, // check only
    pub bpb_tot_sec_16: u16,   // check only
    pub bpb_media: u8,         // `unused`
    pub bpb_fat_sz_16: u16,    // check only
    // BPB_SecPerTrk
    // BPB_NumHeads
    // BPB_HiddSec
    pub bpb_tot_sec_32: u32,

    // > 36-511
    pub bpb_fat_sz_32: u32,
    // BPB_ExtFlags
    pub bpb_fs_ver: u16, // `unused`
    pub bpb_root_clus: u32,
    pub bpb_fs_info: u16, // `unused` temporarily
    pub bpb_bk_boot_sec: u16,
    // BPB_Reserved
    // BS_DrvNum
    // BS_Reserved
    pub bs_boot_sig: u8, // `unused`
    // BS_VolID
    // BS_VolLab
    pub bs_fil_sys_type: [u8; 8],   // `unused`
    pub bs_boot_code_32: [u8; 420], // `unused`
    pub bs_boot_sign: u16,          // check only
}

#[allow(dead_code)]
impl BootSec {
    pub fn new(buf: &mut [u8; 512]) -> Result<Self, scroll::Error> {
        // TODO
        Ok(BootSec {
            bs_oem_name: buf.pread_with(3, LE)?,
            bpb_byts_per_sec: buf.pread_with(11, LE)?,
            bpb_sec_per_clus: buf.pread_with(13, LE)?,
            bpb_rsvd_sec_cnt: buf.pread_with(14, LE)?,
            bpb_num_fats: buf.pread_with(16, LE)?,
            bpb_root_ent_cnt: buf.pread_with(17, LE)?,
            bpb_tot_sec_16: buf.pread_with(19, LE)?,
            bpb_media: buf.pread_with(21, LE)?,
            bpb_fat_sz_16: buf.pread_with(22, LE)?,
            bpb_tot_sec_32: buf.pread_with(32, LE)?,

            bpb_fat_sz_32: buf.pread_with(36, LE)?,
            bpb_fs_ver: buf.pread_with(42, LE)?,
            bpb_root_clus: buf.pread_with(44, LE)?,
            bpb_fs_info: buf.pread_with(48, LE)?,
            bpb_bk_boot_sec: buf.pread_with(50, LE)?,
            bs_boot_sig: buf.pread_with(66, LE)?,
            bs_fil_sys_type: buf.pread_with(82, LE)?,
            bs_boot_code_32: buf.pread_with(90, LE)?,
            bs_boot_sign: buf.pread_with(510, LE)?,
        })
    }

    pub fn fat_start_sector(&self) -> u16 {
        self.bpb_rsvd_sec_cnt
    }

    pub fn fat_sectors(&self) -> u32 {
        self.bpb_fat_sz_32 * self.bpb_num_fats as u32
    }

    // >> UNUSED
    // fn root_dir_start_sector(&self) -> u32 {
    //     self.fat_start_sector() as u32 + self.fat_sectors()
    // }
    // fn root_dir_sectors(&self) -> u32 {
    //     (32 * self.bpb_root_ent_cnt as u32 + self.bpb_byts_per_sec as u32 - 1)
    //         / self.bpb_byts_per_sec as u32
    // }
    // << UNUSED

    pub fn data_start_sector(&self) -> u32 {
        self.fat_start_sector() as u32 + self.fat_sectors()
    }

    pub fn data_sectors(&self) -> u32 {
        self.bpb_tot_sec_32 - self.data_start_sector()
    }

    pub fn cluster_size(&self) -> u32 {
        self.bpb_byts_per_sec as u32 * self.bpb_sec_per_clus as u32
    }

    pub fn check_fat32(&self) {
        assert!(self.bs_boot_sign == 0xAA55);

        assert!(self.bpb_sec_per_clus != 0);
        let num_clusters = self.data_sectors() / self.bpb_sec_per_clus as u32;
        assert!(num_clusters >= 65526);

        // temporarily only support sector size 512
        assert!(self.bpb_byts_per_sec as usize == 512);
        assert!(self.bpb_num_fats == 2);
    }
}

#[derive(Debug)]
pub enum FatEnt {
    Eoc,
    Bad,
    Unused,
    Reserved,
    Next(ClusNo),
}

impl FatEnt {
    // const SZ: u8 = 4;
    pub fn new(buf: &[u8]) -> Self {
        let buf_3 = buf[3] & 0x0F;
        if buf_3 == 0 && buf[2] == 0 && buf[1] == 0 {
            if buf[0] == 0 {
                return FatEnt::Unused;
            } else if buf[0] == 1 {
                return FatEnt::Reserved;
            }
        } else if buf_3 == 0x0F && buf[2] == 0xFF && buf[1] == 0xFF {
            if buf[0] >= 0xF8 {
                return FatEnt::Eoc;
            } else if buf[0] == 0xF7 {
                return FatEnt::Bad;
            }
        }
        FatEnt::Next(u32::from_le_bytes([buf[0], buf[1], buf[2], buf_3]))
    }
}

#[derive(Debug)]
pub struct Date {
    pub year: u8,
    pub month: u8,
    pub day: u8,
}

impl From<u16> for Date {
    fn from(val: u16) -> Self {
        let year = (val >> 9) as u8;
        let month = (val >> 5 & 0xF) as u8;
        let day = (val & 0x1F) as u8;
        Date { year, month, day }
    }
}

impl From<Date> for u16 {
    fn from(date: Date) -> Self {
        (date.year as u16) << 9 | ((date.month & 0xF) as u16) << 5 | (date.day & 0x1F) as u16
    }
}

#[derive(Debug)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl From<u16> for Time {
    fn from(val: u16) -> Self {
        let hour = (val >> 11) as u8;
        let minute = (val >> 5 & 0x3F) as u8;
        let second = (val & 0x1F) as u8;
        Time {
            hour,
            minute,
            second,
        }
    }
}

impl From<Time> for u16 {
    fn from(time: Time) -> Self {
        (time.hour as u16) << 11 | ((time.minute & 0x3F) as u16) << 5 | (time.second & 0x1F) as u16
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct DirEntSfn {
    name: [u8; 11],
    attr: u8,
    nt_res: u8,
    crt_time_tenth: u8,
    crt_time: u16,
    crt_date: u16,
    lst_acc_date: u16, // `unused` temporarily
    fst_clus_hi: u16,
    wrt_time: u16,
    wrt_date: u16,
    fst_clus_lo: u16,
    pub file_size: u32,

    // the on-disk position (clus_no and offset) of this entry
    pub clus_no: ClusNo,
    pub off: u32,
}

#[allow(dead_code)]
impl DirEntSfn {
    // refer to [2]
    const BODY_LOW_CASE: u8 = 0x08;

    pub fn create_chksum(&self) -> u8 {
        (0..11).fold(0u8, |sum, i| {
            self.name[i].wrapping_add(sum >> 1).wrapping_add(sum << 7)
        })
    }

    // `imprecise`
    pub fn name(&self) -> String {
        let mut name = self.name;
        if name[0] == 0x05 {
            name[0] = 0xE5;
        };

        let mut res = String::new();
        for &ch in name.iter().take(8) {
            if ch == b' ' {
                break;
            }
            res.push(ch.into());
        }

        let mut ext_str = String::new();
        for &ch in name.iter().skip(8) {
            if ch != b' ' {
                ext_str.push(ch.into());
            }
        }

        if !ext_str.is_empty() {
            res.push('.');
            res.push_str(&ext_str);
        }

        // `uncertain`
        if self.nt_res & Self::BODY_LOW_CASE != 0 {
            res.to_lowercase()
        } else {
            res
        }
    }

    // `imprecise`
    pub fn volume_label(&self) -> String {
        match String::from_utf8(self.name.to_vec()) {
            Ok(name) => name.trim_end().to_owned(),
            Err(_) => String::from("ERROR"),
        }
    }

    pub fn fst_clus(&self) -> u32 {
        (self.fst_clus_hi as u32) << 16 | self.fst_clus_lo as u32
    }

    pub fn is_unused(&self) -> bool {
        self.name[0] == 0xE5 || self.is_end()
    }

    pub fn is_end(&self) -> bool {
        self.name[0] == 0x00
    }

    pub fn is_rdonly(&self) -> bool {
        self.attr & DirEnt::ATTR_READ_ONLY != 0
    }

    pub fn is_hidden(&self) -> bool {
        self.attr & DirEnt::ATTR_HIDDEN != 0
    }

    pub fn is_system(&self) -> bool {
        self.attr & DirEnt::ATTR_SYSTEM != 0
    }

    pub fn is_volumeid(&self) -> bool {
        self.attr & DirEnt::ATTR_VOLUME_ID != 0
    }

    pub fn is_dir(&self) -> bool {
        self.attr & DirEnt::ATTR_DIRECTORY != 0
    }

    pub fn is_archive(&self) -> bool {
        self.attr & DirEnt::ATTR_ARCHIVE != 0
    }

    fn make_dt(date: &Date, time: &Time) -> Option<SystemTime> {
        Some(
            Local
                .with_ymd_and_hms(
                    1980 + date.year as i32,
                    date.month.into(),
                    date.day.into(),
                    time.hour.into(),
                    time.minute.into(),
                    time.second.into(),
                )
                .single()?
                .into(),
        )
    }

    pub fn wrt_time(&self) -> SystemTime {
        if let Some(time) = Self::make_dt(&self.wrt_date.into(), &self.wrt_time.into()) {
            time
        } else {
            SystemTime::UNIX_EPOCH
        }
    }

    pub fn crt_time(&self) -> SystemTime {
        if let Some(time) = Self::make_dt(&self.crt_date.into(), &self.crt_time.into()) {
            let tenth_sec = self.crt_time_tenth / 100;
            let tenth_milsec = self.crt_time_tenth % 100;
            time + std::time::Duration::new(tenth_sec.into(), tenth_milsec as u32 * 1000_1000)
        } else {
            SystemTime::UNIX_EPOCH
        }
    }

    pub fn last_acc_time(&self) -> SystemTime {
        if let Some(time) = Self::make_dt(&self.crt_date.into(), &0.into()) {
            time
        } else {
            SystemTime::UNIX_EPOCH
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct DirEntLfn {
    ord: u8,
    name1: [u16; 5],
    attr: u8,
    typ: u8, // `unused` zero
    pub chksum: u8,
    name2: [u16; 6],
    fst_clus_lo: u16, // `unused` zero
    name3: [u16; 2],
}

#[allow(dead_code)]
impl DirEntLfn {
    // `imprecise`
    pub fn name(&self) -> String {
        let mut bytes: Vec<u16> = Vec::new();
        bytes.extend_from_slice(&self.name1);
        bytes.extend_from_slice(&self.name2);
        bytes.extend_from_slice(&self.name3);
        let mut term_idx: usize = bytes.len();
        for (i, &c) in bytes.iter().enumerate() {
            if c == 0x0000u16 {
                term_idx = i;
                break;
            }
        }
        String::from_utf16_lossy(&bytes[0..term_idx])
    }

    pub fn is_last(&self) -> bool {
        self.ord & 0x40 != 0
    }

    pub fn ordno(&self) -> u8 {
        self.ord & 0x3F
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum DirEnt {
    Sfn(DirEntSfn),
    Lfn(DirEntLfn),
}

#[allow(dead_code)]
impl DirEnt {
    const ATTR_READ_ONLY: u8 = 0x01;
    const ATTR_HIDDEN: u8 = 0x02;
    const ATTR_SYSTEM: u8 = 0x04;
    const ATTR_VOLUME_ID: u8 = 0x08;
    const ATTR_DIRECTORY: u8 = 0x10;
    const ATTR_ARCHIVE: u8 = 0x20;
    const ATTR_LONG_FILE_NAME: u8 = 0x0F;

    pub const SZ: u32 = 32;

    pub fn new(buf: &[u8], clus_no: ClusNo, offset: u32) -> Result<Self, scroll::Error> {
        let attr: u8 = buf.pread_with(11, LE)?;
        if attr == Self::ATTR_LONG_FILE_NAME {
            Ok(DirEnt::Lfn(DirEntLfn {
                ord: buf.pread_with(0, LE)?,
                name1: buf.pread_with(1, LE)?,
                attr: buf.pread_with(11, LE)?,
                typ: buf.pread_with(12, LE)?,
                chksum: buf.pread_with(13, LE)?,
                name2: buf.pread_with(14, LE)?,
                fst_clus_lo: buf.pread_with(26, LE)?,
                name3: buf.pread_with(28, LE)?,
            }))
        } else {
            Ok(DirEnt::Sfn(DirEntSfn {
                name: buf.pread_with(0, LE)?,
                attr: buf.pread_with(11, LE)?,
                nt_res: buf.pread_with(12, LE)?,
                crt_time_tenth: buf.pread_with(13, LE)?,
                crt_time: buf.pread_with(14, LE)?,
                crt_date: buf.pread_with(16, LE)?,
                lst_acc_date: buf.pread_with(18, LE)?,
                fst_clus_hi: buf.pread_with(20, LE)?,
                wrt_time: buf.pread_with(22, LE)?,
                wrt_date: buf.pread_with(24, LE)?,
                fst_clus_lo: buf.pread_with(26, LE)?,
                file_size: buf.pread_with(28, LE)?,
                clus_no,
                off: offset,
            }))
        }
    }
}
