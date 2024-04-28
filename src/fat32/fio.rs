use std::{time::SystemTime, vec};

use super::spec::{BootSec, ClusNo, DirEnt, DirEntLfn, FatEnt};

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("Unimplemented")]
    Unimplemented,
    #[error("dir entries reduction failed")]
    DirEntReductionFailure,
}

const SEC_SZ: usize = 512;

pub trait Device {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64);
}

type Sec = [u8; SEC_SZ];
type Clus = Vec<u8>;

struct SecIo {
    start: u64, // sec number
    from: u64,
}

impl SecIo {
    fn read(&self, sec_no: u64, device: &dyn Device) -> Sec {
        let mut buf: Sec = [0u8; SEC_SZ];
        device.read_exact_at(&mut buf, (self.start + self.from + sec_no) * SEC_SZ as u64);
        buf
    }
}

struct ClusIo {
    start: u64, // in bytes
    skip: u32,
    clus_sz: u32,
}

impl ClusIo {
    fn read(&self, clus_no: u32, device: &dyn Device) -> Clus {
        let mut buf = vec![0u8; self.clus_sz as usize];
        device.read_exact_at(
            &mut buf,
            self.start + (self.skip + clus_no - 2) as u64 * self.clus_sz as u64,
        );
        buf
    }
}

struct Fat {
    sec_io: SecIo,
    entries_per_sec: u64,
}

#[allow(dead_code)]
impl Fat {
    const ENT_SZ: usize = 4;
    fn read_one(&self, no: u64, device: &dyn Device) -> FatEnt {
        let sec_no = no / self.entries_per_sec;
        let ent_offset = (no % self.entries_per_sec) as usize;
        let sec = self.sec_io.read(sec_no, device);
        FatEnt::new(&sec[Fat::ENT_SZ * ent_offset..Fat::ENT_SZ * (ent_offset + 1)])
    }

    fn new_iter<'a>(&'a self, device: &'a dyn Device, first_clusno: ClusNo) -> FatIter {
        match self.read_one(first_clusno.into(), device) {
            FatEnt::Eoc | FatEnt::Next(_) => (),
            en => panic!("fs err: trying to iterate a {:#?} Fat entry", en),
        };
        FatIter {
            fat: self,
            device,
            next_clusno: Some(first_clusno),
        }
    }
}

#[derive(Clone)]
struct FatIter<'a> {
    fat: &'a Fat,
    device: &'a dyn Device,
    next_clusno: Option<ClusNo>,
}

impl<'a> Iterator for FatIter<'a> {
    type Item = ClusNo;
    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.next_clusno;
        if let Some(no) = curr {
            self.next_clusno = match self.fat.read_one(no.into(), self.device) {
                FatEnt::Eoc => None,
                FatEnt::Next(no) => Some(no),
                _ => panic!("fs: err0"),
            };
        };
        curr
    }
}

struct ClusIter<'a> {
    fat_iter: FatIter<'a>,
    device: &'a dyn Device,
    clus_io: &'a ClusIo,
}

impl<'a> Iterator for ClusIter<'a> {
    type Item = Clus;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(clusno) = self.fat_iter.next() {
            Some(self.clus_io.read(clusno, self.device))
        } else {
            None
        }
    }
}

#[allow(dead_code)]
pub struct Fio<'a> {
    device: Box<(dyn Device + 'a)>,
    fat: Fat,
    clus_io: ClusIo,
    pub root_clusno: ClusNo,
}

#[allow(dead_code)]
impl<'a> Fio<'a> {
    pub fn new(device: impl Device + 'a) -> Self {
        let mut buf: Sec = [0u8; SEC_SZ];
        device.read_exact_at(&mut buf, 0);

        let sec0 = BootSec::new(&mut buf);
        sec0.check_fat32();
        // temporarily only support sector size 512
        assert!(sec0.bpb_byts_per_sec.value as usize == SEC_SZ);
        assert!(sec0.bpb_num_fats.value == 2);

        let clus_io = ClusIo {
            start: sec0.data_start_sector() as u64 * sec0.bpb_byts_per_sec.value as u64,
            skip: 0,
            clus_sz: sec0.cluster_size(),
        };
        let fat_1 = Fat {
            sec_io: SecIo {
                start: sec0.fat_start_sector().into(),
                from: sec0.bpb_fat_sz_32.value.into(),
            },
            entries_per_sec: sec0.bpb_byts_per_sec.value as u64 / Fat::ENT_SZ as u64,
        };
        Fio {
            device: Box::new(device),
            fat: fat_1,
            clus_io,
            root_clusno: sec0.bpb_root_clus.value,
        }
    }

    pub fn read_dirents(&self, first_clusno: ClusNo) -> Vec<File> {
        assert!(first_clusno >= 2);
        let mut res: Vec<File> = vec![];
        let fat_iter = self.fat.new_iter(self.device.as_ref(), first_clusno);
        let clus_iter = ClusIter {
            fat_iter: fat_iter.clone(),
            device: self.device.as_ref(),
            clus_io: &self.clus_io,
        };
        let mut ents: Vec<DirEnt> = vec![];
        for (clus, clus_no) in clus_iter.zip(fat_iter) {
            for (off, buf) in clus.chunks(DirEnt::SZ as usize).enumerate() {
                match DirEnt::new(buf, clus_no, off as u32) {
                    dirent @ DirEnt::Lfn(_) => {
                        ents.push(dirent);
                    }
                    DirEnt::Sfn(en) => {
                        if en.is_end() {
                            ents.clear();
                            break;
                        }
                        ents.push(DirEnt::Sfn(en));
                        if let Ok(file) = File::try_from(ents) {
                            res.push(file)
                        };
                        ents = vec![];
                    }
                };
            }
        }
        res
    }

    pub fn readroot(&self) -> Vec<File> {
        self.read_dirents(self.root_clusno)
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub id: u64, // a unique id consists of entry's clus_no and offset
    pub name: String,
    pub is_rdonly: bool,
    pub is_hidden: bool,
    pub is_system: bool,
    pub is_dir: bool,
    // pub is_archive: bool,
    pub size: u32,
    pub fst_clus: u32,
    pub crt_time: SystemTime,
    pub wrt_time: SystemTime,
}

impl TryFrom<Vec<DirEnt>> for File {
    type Error = FsError;
    fn try_from(mut ents: Vec<DirEnt>) -> Result<Self, Self::Error> {
        // consume the sfn
        let sfn = match ents.pop() {
            Some(DirEnt::Sfn(en)) => en,
            _ => panic!("fs::File: try_from"),
        };
        if sfn.is_unused() || sfn.is_volumeid() {
            return Err(FsError::DirEntReductionFailure);
        }
        let chksum = sfn.create_chksum();

        let mut name = sfn.name();

        // process lfn and build name if valid
        if !ents.is_empty() && ents.len() <= 20 {
            // extract
            let lfns: Vec<&DirEntLfn> = ents
                .iter()
                .map(|dirent| match dirent {
                    DirEnt::Lfn(en) => en,
                    _ => panic!("fs::File: try_from"),
                })
                .collect();

            'check: {
                if let Some(en) = lfns.first() {
                    if !en.is_last() {
                        break 'check;
                    }
                } else {
                    break 'check;
                }
                let mut longname = String::new();
                // checksum and build name
                for &en in lfns.iter() {
                    if en.chksum.value != chksum {
                        break 'check;
                    }
                    longname.insert_str(0, &en.name());
                }
                // check order
                if lfns
                    .iter()
                    .try_fold(ents.len() + 1, |acc, &en| {
                        if acc - 1 == en.ordno().into() {
                            Ok(acc - 1)
                        } else {
                            Err(0)
                        }
                    })
                    .is_err()
                {
                    break 'check;
                }

                name = longname;
            }
        }
        Ok(File {
            id: (sfn.off as u64) << 32 | sfn.clus_no as u64,
            name,
            is_rdonly: sfn.is_rdonly(),
            is_dir: sfn.is_dir(),
            is_hidden: sfn.is_hidden(),
            is_system: sfn.is_system(),
            size: sfn.file_size.value,
            fst_clus: sfn.fst_clus(),
            crt_time: sfn.crt_time(),
            wrt_time: sfn.wrt_time(),
        })
    }
}
