use std::{cmp::min, io::SeekFrom, time::SystemTime, vec};

use super::spec::{BootSec, ClusNo, DirEnt, DirEntLfn, FatEnt};
use crate::device::Device;

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("Unimplemented")]
    Unimplemented,
    #[error("dir entries reduction failed")]
    DirEntReductionFailure,
}

const SEC_SZ: usize = 512;
type Sec = [u8; SEC_SZ];
type Clus = Vec<u8>;

struct SecIo {
    base: u64, // sec number
    skip: u64, // secs
}

impl SecIo {
    fn read(&self, sec_no: u64, device: &mut dyn Device) -> Sec {
        let mut buf: Sec = [0u8; SEC_SZ];
        device
            .seek(SeekFrom::Start(
                (self.base + self.skip + sec_no) * SEC_SZ as u64,
            ))
            .unwrap();
        device.read_exact(&mut buf).unwrap();
        buf
    }
}

struct ClusIo {
    start: u64, // in bytes
    skip: u32,
    clus_sz: u32,
}

impl ClusIo {
    fn read(&self, clus_no: u32, device: &mut dyn Device) -> Clus {
        let mut buf = vec![0u8; self.clus_sz as usize];
        device
            .seek(SeekFrom::Start(
                self.start + (self.skip + clus_no - 2) as u64 * self.clus_sz as u64,
            ))
            .unwrap();
        device.read_exact(&mut buf).unwrap();
        buf
    }

    fn read_all(&self, fats: Vec<ClusNo>, device: &mut dyn Device) -> Vec<Clus> {
        fats.into_iter()
            .map(|clusno| self.read(clusno, device))
            .collect()
    }
}

struct Fat {
    sec_io: SecIo,
    entries_per_sec: u64,
}

impl Fat {
    const ENT_SZ: usize = 4;
    fn read_one(&self, no: u64, device: &mut dyn Device) -> FatEnt {
        let sec_no = no / self.entries_per_sec;
        let ent_offset = (no % self.entries_per_sec) as usize;
        let sec = self.sec_io.read(sec_no, device);
        FatEnt::new(&sec[Fat::ENT_SZ * ent_offset..Fat::ENT_SZ * (ent_offset + 1)])
    }

    fn read_all(&self, device: &mut dyn Device, first_clusno: ClusNo) -> Vec<ClusNo> {
        self.new_iter(device, first_clusno).collect()
    }

    fn new_iter<'a>(&'a self, device: &'a mut dyn Device, first_clusno: ClusNo) -> FatIter {
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

struct FatIter<'a> {
    fat: &'a Fat,
    device: &'a mut dyn Device,
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

#[allow(dead_code)]
pub struct Fio<'a> {
    device: Box<(dyn Device + 'a)>,
    fat: Fat,
    clus_io: ClusIo,
    pub root_clusno: ClusNo,
    clus_sz: u32,
    pub bootsec: BootSec,
}

#[allow(dead_code)]
impl<'a> Fio<'a> {
    pub fn new(mut device: impl Device + 'a) -> Self {
        let mut buf: Sec = [0u8; SEC_SZ];
        device.seek(SeekFrom::Start(0)).unwrap();
        device.read_exact(&mut buf).unwrap();

        let bootsec = BootSec::new(&mut buf).unwrap();
        bootsec.check_fat32();

        let clus_io = ClusIo {
            start: bootsec.data_start_sector() as u64 * bootsec.bpb_byts_per_sec as u64,
            skip: 0,
            clus_sz: bootsec.cluster_size(),
        };
        let fat_1 = Fat {
            sec_io: SecIo {
                base: bootsec.fat_start_sector().into(),
                skip: bootsec.bpb_fat_sz_32.into(),
            },
            entries_per_sec: bootsec.bpb_byts_per_sec as u64 / Fat::ENT_SZ as u64,
        };
        Fio {
            device: Box::new(device),
            fat: fat_1,
            clus_io,
            root_clusno: bootsec.bpb_root_clus,
            clus_sz: bootsec.cluster_size(),
            bootsec,
        }
    }

    pub fn read_clus(&mut self, clusno: ClusNo) -> Clus {
        self.clus_io.read(clusno, self.device.as_mut())
    }

    pub fn read_dirents(&mut self, first_clusno: ClusNo) -> Vec<Finfo> {
        if first_clusno == 0 {
            // a empty dir entry has first_clusno set to 0
            return vec![];
        }
        assert!(first_clusno != 1);
        let mut res: Vec<Finfo> = vec![];
        let fats = self.fat.read_all(self.device.as_mut(), first_clusno);
        // let mut fat_iter = self.fat.new_iter(self.device.as_mut(), first_clusno);
        let mut ents: Vec<DirEnt> = vec![];
        for clus_no in fats.into_iter() {
            let clus = self.clus_io.read(clus_no, self.device.as_mut());
            for (off, buf) in clus.chunks(DirEnt::SZ as usize).enumerate() {
                match DirEnt::new(buf, clus_no, off as u32) {
                    Ok(dirent @ DirEnt::Lfn(_)) => {
                        ents.push(dirent);
                    }
                    Ok(DirEnt::Sfn(en)) => {
                        if en.is_end() {
                            ents.clear();
                            break;
                        }
                        ents.push(DirEnt::Sfn(en));
                        if let Ok(file) = Finfo::try_from(ents) {
                            res.push(file)
                        };
                        ents = vec![];
                    }
                    Err(_) => panic!("[fio] read_dirents: failed."),
                };
            }
        }
        res
    }

    pub fn readroot(&mut self) -> Vec<Finfo> {
        self.read_dirents(self.root_clusno)
    }

    pub fn readfile(&mut self, fi: &Finfo, offset: u32, size: u32) -> Vec<u8> {
        if offset >= fi.size || size == 0 {
            return vec![];
        }
        let sz = min(size, fi.size - offset);
        let start_clus = offset / self.clus_sz;
        let start_off = (offset % self.clus_sz) as usize;
        let end_clus = (offset + sz - 1) / self.clus_sz;

        let fats: Vec<ClusNo> = self
            .fat
            .new_iter(self.device.as_mut(), fi.fst_clus)
            .skip(start_clus as usize)
            .take((end_clus - start_clus + 1) as usize)
            .collect();

        let bytes: Vec<u8> = self.clus_io.read_all(fats, self.device.as_mut()).concat();
        println!(
            "[fio] readfile: file({}) off({offset}) size({sz}) got({})",
            fi.name,
            bytes.len()
        );
        bytes[start_off..(start_off + sz as usize)].to_vec()
    }
}

#[derive(Debug, Clone)]
pub struct Finfo {
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

impl TryFrom<Vec<DirEnt>> for Finfo {
    type Error = FsError;
    fn try_from(mut ents: Vec<DirEnt>) -> Result<Self, Self::Error> {
        // consume the sfn
        let sfn = match ents.pop() {
            Some(DirEnt::Sfn(en)) => en,
            _ => panic!("fs::Finfo: try_from"),
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
                    _ => panic!("fs::Finfo: try_from"),
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
                    if en.chksum != chksum {
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
        Ok(Finfo {
            id: (sfn.off as u64) << 32 | sfn.clus_no as u64,
            name,
            is_rdonly: sfn.is_rdonly(),
            is_dir: sfn.is_dir(),
            is_hidden: sfn.is_hidden(),
            is_system: sfn.is_system(),
            size: sfn.file_size,
            fst_clus: sfn.fst_clus(),
            crt_time: sfn.crt_time(),
            wrt_time: sfn.wrt_time(),
        })
    }
}
