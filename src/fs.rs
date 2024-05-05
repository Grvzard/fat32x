use core::panic;
use std::{collections::BTreeMap, rc::Rc, vec};

use crate::device::Device;
use crate::fat32::fio::{Finfo, Fio};

type DirMap = BTreeMap<u64, Vec<Rc<Finfo>>>;
type FinfoMap = BTreeMap<u64, Rc<Finfo>>;

// #[allow(dead_code)]
pub struct Fs<'a> {
    fio: Fio<'a>,
    dirmap: DirMap,
    fmap: FinfoMap,
    filesopen: BTreeMap<u64, u32>,
}

// #[allow(dead_code)]
impl<'a> Fs<'a> {
    pub fn new(device: impl Device + 'a) -> Self {
        let fio = Fio::new(device);
        let dirmap = DirMap::new();
        let fmap = FinfoMap::new();
        let mut fs = Fs {
            fio,
            dirmap,
            fmap,
            filesopen: BTreeMap::new(),
        };
        let rootfiles: Vec<Rc<Finfo>> = fs
            .fio
            .read_dirents(fs.fio.root_clusno)
            .into_iter()
            .map(Rc::new)
            .collect();

        rootfiles.iter().for_each(|rc_fi| {
            fs.fmap.insert(rc_fi.id, rc_fi.clone());
        });

        fs.dirmap.insert(1, rootfiles);
        fs
    }

    pub fn readdir(&mut self, id: u64) -> &Vec<Rc<Finfo>> {
        if self.dirmap.get(&id).is_none() {
            if let Some(di) = self.fmap.get(&id) {
                let rc_files = if di.fst_clus != 0 {
                    self.fio
                        .read_dirents(di.fst_clus)
                        .into_iter()
                        .map(Rc::new)
                        .collect()
                } else {
                    vec![]
                };
                rc_files.iter().for_each(|rc_fi| {
                    self.fmap.insert(rc_fi.id, rc_fi.clone());
                });
                self.dirmap.insert(id, rc_files);
            } else {
                panic!("fs: readdir")
            }
        }
        &self.dirmap[&id]
    }

    pub fn lookup(&mut self, parent: u64, name: &str) -> Option<Rc<Finfo>> {
        for fi in self.readdir(parent) {
            if fi.name == name {
                return Some(fi.clone());
            }
        }
        None
    }

    pub fn getinfo(&mut self, id: u64) -> Option<Rc<Finfo>> {
        self.fmap.get(&id).cloned()
    }

    pub fn open(&mut self, id: u64) -> bool {
        if let Some(_fi) = self.getinfo(id) {
            println!("open: {:?}", _fi.name);
            self.filesopen
                .entry(id)
                .and_modify(|cnt| *cnt += 1)
                .or_insert(1);
            true
        } else {
            false
        }
    }

    pub fn close(&mut self, id: u64) {
        if let Some(cnt) = self.filesopen.get_mut(&id) {
            *cnt -= 1;
            if *cnt == 0 {
                self.filesopen.remove(&id);
            }
        }
    }

    pub fn read(&mut self, id: u64, offset: u32, size: u32) -> Option<Vec<u8>> {
        self.fmap
            .get(&id)
            .map(|fi| self.fio.readfile(fi, offset, size))
    }
}
