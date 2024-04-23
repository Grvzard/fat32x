// References:
// [1] https://download.microsoft.com/download/1/6/1/161ba512-40e2-4cc9-843a-923143f3456c/fatgen103.doc
// [2] http://elm-chan.org/docs/fat_e.html
// [3] https://en.wikipedia.org/wiki/Design_of_the_FAT_file_system#FAT

use super::field::{BytesField, Field, U16Field, U32Field, U8Field, Utf16Field};

pub type ClusNo = u32;

pub struct BootSec {
    // > 0-35
    // BS_JmpBoot
    pub bs_oem_name: BytesField<3, 8>, // `unused`
    pub bpb_byts_per_sec: U16Field<11>,
    pub bpb_sec_per_clus: U8Field<13>,
    pub bpb_rsvd_sec_cnt: U16Field<14>,
    pub bpb_num_fats: U8Field<16>,
    pub bpb_root_ent_cnt: U16Field<17>, // check only
    pub bpb_tot_sec_16: U16Field<19>,   // check only
    pub bpb_media: U8Field<21>,         // `unused`
    pub bpb_fat_sz_16: U16Field<22>,    // check only
    // BPB_SecPerTrk
    // BPB_NumHeads
    // BPB_HiddSec
    pub bpb_tot_sec_32: U32Field<32>,

    // > 36-511
    pub bpb_fat_sz_32: U32Field<36>,
    // BPB_ExtFlags
    pub bpb_fs_ver: U16Field<42>, // `unused`
    pub bpb_root_clus: U32Field<44>,
    pub bpb_fs_info: U16Field<48>,
    pub bpb_bk_boot_sec: U16Field<50>,
    // BPB_Reserved
    // BS_DrvNum
    // BS_Reserved
    pub bs_boot_sig: U8Field<66>, // `unused`
    // BS_VolID
    // BS_VolLab
    pub bs_fil_sys_type: BytesField<82, 8>,   // `unused`
    pub bs_boot_code_32: BytesField<90, 420>, // `unused`
    pub bs_boot_sign: U16Field<510>,          // check only
}

#[allow(dead_code)]
impl BootSec {
    pub fn new(buf: &mut [u8; 512]) -> Self {
        BootSec {
            bs_oem_name: Field::load(buf),
            bpb_byts_per_sec: Field::load(buf),
            bpb_sec_per_clus: Field::load(buf),
            bpb_rsvd_sec_cnt: Field::load(buf),
            bpb_num_fats: Field::load(buf),
            bpb_root_ent_cnt: Field::load(buf),
            bpb_tot_sec_16: Field::load(buf),
            bpb_media: Field::load(buf),
            bpb_fat_sz_16: Field::load(buf),
            bpb_tot_sec_32: Field::load(buf),

            bpb_fat_sz_32: Field::load(buf),
            bpb_fs_ver: Field::load(buf),
            bpb_root_clus: Field::load(buf),
            bpb_fs_info: Field::load(buf),
            bpb_bk_boot_sec: Field::load(buf),
            bs_boot_sig: Field::load(buf),
            bs_fil_sys_type: Field::load(buf),
            bs_boot_code_32: Field::load(buf),
            bs_boot_sign: Field::load(buf),
        }
    }

    pub fn fat_start_sector(&self) -> u16 {
        self.bpb_rsvd_sec_cnt.value
    }

    pub fn fat_sectors(&self) -> u32 {
        self.bpb_fat_sz_32.value * self.bpb_num_fats.value as u32
    }

    // >> UNUSED
    // fn root_dir_start_sector(&self) -> u32 {
    //     self.fat_start_sector() as u32 + self.fat_sectors()
    // }
    // fn root_dir_sectors(&self) -> u32 {
    //     (32 * self.bpb_root_ent_cnt.value as u32 + self.bpb_byts_per_sec.value as u32 - 1)
    //         / self.bpb_byts_per_sec.value as u32
    // }
    // << UNUSED

    pub fn data_start_sector(&self) -> u32 {
        self.fat_start_sector() as u32 + self.fat_sectors()
    }

    pub fn data_sectors(&self) -> u32 {
        self.bpb_tot_sec_32.value - self.data_start_sector() as u32
    }

    pub fn cluster_size(&self) -> u32 {
        self.bpb_byts_per_sec.value as u32 * self.bpb_sec_per_clus.value as u32
    }

    pub fn check_fat32(&self) {
        assert_eq!(self.bs_boot_sign.value, 0xAA55);

        let num_clusters = self.data_sectors() / self.bpb_sec_per_clus.value as u32;
        assert!(num_clusters >= 65526);
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

#[allow(dead_code)]
#[derive(Debug)]
pub struct DirEntSfn {
    name: BytesField<0, 11>,
    attr: U8Field<11>,
    nt_res: U8Field<12>,         // `temporarily unused`
    crt_time_tenth: U8Field<13>, // `temporarily unused`
    crt_time: U16Field<14>,      // `temporarily unused`
    crt_date: U16Field<16>,      // `temporarily unused`
    lst_acc_date: U16Field<18>,  // `temporarily unused`
    fst_clus_hi: U16Field<20>,
    wrt_time: U16Field<22>, // `temporarily unused`
    wrt_date: U16Field<24>, // `temporarily unused`
    fst_clus_lo: U16Field<26>,
    pub file_size: U32Field<28>,
}

#[allow(dead_code)]
impl DirEntSfn {
    // refer to [2]
    const BODY_LOW_CASE: u8 = 0x08;

    pub fn create_chksum(&self) -> u8 {
        (0..11).fold(0u8, |sum, i| {
            self.name.value[i]
                .wrapping_add(sum >> 1)
                .wrapping_add(sum << 7)
        })
    }

    // `imprecise`
    pub fn name(&self) -> String {
        let mut name = self.name.value.clone();
        if name[0] == 0x05 {
            name[0] = 0xE5;
        };

        let mut res = String::new();
        for i in 0..8 {
            if name[i] == b' ' {
                break;
            }
            res.push(name[i].into());
        }

        let mut ext_str = String::new();
        for i in 8..11 {
            if name[i] != b' ' {
                ext_str.push(name[i].into());
            }
        }

        if ext_str.len() > 0 {
            res.push('.');
            res.push_str(&ext_str);
        }

        // `uncertain`
        if self.nt_res.value & Self::BODY_LOW_CASE != 0 {
            res.to_lowercase()
        } else {
            res
        }
    }

    // `imprecise`
    pub fn volume_label(&self) -> String {
        match String::from_utf8(self.name.value.to_vec()) {
            Ok(name) => name.trim_end().to_owned(),
            Err(_) => String::from("ERROR"),
        }
    }

    pub fn fst_clus(&self) -> u32 {
        (self.fst_clus_hi.value as u32) << 16 | self.fst_clus_lo.value as u32
    }

    pub fn is_unused(&self) -> bool {
        self.name.value[0] == 0xE5 || self.is_end()
    }

    pub fn is_end(&self) -> bool {
        self.name.value[0] == 0x00
    }

    pub fn is_rdonly(&self) -> bool {
        self.attr.value & DirEnt::ATTR_READ_ONLY != 0
    }

    pub fn is_hidden(&self) -> bool {
        self.attr.value & DirEnt::ATTR_HIDDEN != 0
    }

    pub fn is_system(&self) -> bool {
        self.attr.value & DirEnt::ATTR_SYSTEM != 0
    }

    pub fn is_volumeid(&self) -> bool {
        self.attr.value & DirEnt::ATTR_VOLUME_ID != 0
    }

    pub fn is_dir(&self) -> bool {
        self.attr.value & DirEnt::ATTR_DIRECTORY != 0
    }

    pub fn is_archive(&self) -> bool {
        self.attr.value & DirEnt::ATTR_ARCHIVE != 0
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct DirEntLfn {
    ord: U8Field<0>,
    name1: Utf16Field<1, 5>,
    attr: U8Field<11>,
    typ: U8Field<12>, // `unused` zero
    pub chksum: U8Field<13>,
    name2: Utf16Field<14, 6>,
    fst_clus_lo: U16Field<26>, // `unused` zero
    name3: Utf16Field<28, 2>,
}

#[allow(dead_code)]
impl DirEntLfn {
    // `imprecise`
    pub fn name(&self) -> String {
        let mut bytes: Vec<u16> = Vec::new();
        bytes.extend_from_slice(&self.name1.value);
        bytes.extend_from_slice(&self.name2.value);
        bytes.extend_from_slice(&self.name3.value);
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
        self.ord.value & 0x40 != 0
    }

    pub fn ordno(&self) -> u8 {
        self.ord.value & 0x3F
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

    pub fn new(buf: &[u8]) -> Self {
        let attr: U8Field<11> = Field::load(buf);
        if attr.value == Self::ATTR_LONG_FILE_NAME {
            DirEnt::Lfn(DirEntLfn {
                ord: Field::load(buf),
                name1: Field::load(buf),
                attr: Field::load(buf),
                typ: Field::load(buf),
                chksum: Field::load(buf),
                name2: Field::load(buf),
                fst_clus_lo: Field::load(buf),
                name3: Field::load(buf),
            })
        } else {
            DirEnt::Sfn(DirEntSfn {
                name: Field::load(buf),
                attr: Field::load(buf),
                nt_res: Field::load(buf),
                crt_time_tenth: Field::load(buf),
                crt_time: Field::load(buf),
                crt_date: Field::load(buf),
                lst_acc_date: Field::load(buf),
                fst_clus_hi: Field::load(buf),
                wrt_time: Field::load(buf),
                wrt_date: Field::load(buf),
                fst_clus_lo: Field::load(buf),
                file_size: Field::load(buf),
            })
        }
    }
}
