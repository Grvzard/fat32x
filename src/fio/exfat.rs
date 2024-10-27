// References:
// [1] https://learn.microsoft.com/en-us/windows/win32/fileio/exfat-specification

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("dir entries reduction failed")]
    DirEntReductionFailed,
    // #[error("dir entries read failed")]
    // DirEntReadFailed,
    // #[error("scroll read failed")]
    // Scroll(#[from] scroll::Error),
    // #[error("undefined entry type 0x{0:X}")]
    // UndefinedDirEntry(u8),
}

use std::io::{Read, Seek, SeekFrom};

use scroll::{Pread, LE};

use crate::fio::{self, Finfo};
use crate::spec::exfat::{
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

        let (ent_file, ent_clusno, ent_off, ent_stream) = match (&ents[0], &ents[1]) {
            (EntrySet::FileOrDir(ent0, (ent_clusno, ent_off)), EntrySet::StreamExt(ent1)) => {
                (ent0, *ent_clusno, *ent_off, ent1)
            }
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
            id: (ent_off as u64) << 32 | ent_clusno as u64,
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
