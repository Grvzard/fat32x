use core::panic;
use std::{collections::BTreeMap, rc::Rc, vec};

use super::fio::{Device, Finfo, Fio};

type DirMap = BTreeMap<u64, Vec<Rc<Finfo>>>;
type FinfoMap = BTreeMap<u64, Rc<Finfo>>;

// #[allow(dead_code)]
pub struct Fs<'a> {
    fio: Fio<'a>,
    dirmap: DirMap,
    fmap: FinfoMap,
}

// #[allow(dead_code)]
impl<'a> Fs<'a> {
    pub fn new(device: impl Device + 'a) -> Self {
        let fio = Fio::new(device);
        let dirmap = DirMap::new();
        let fmap = FinfoMap::new();
        let mut fs = Fs { fio, dirmap, fmap };
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
}
