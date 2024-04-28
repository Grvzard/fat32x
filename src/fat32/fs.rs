use std::collections::BTreeMap;

use super::fio::{Device, Finfo, Fio};

type DirMap = BTreeMap<u64, Vec<Finfo>>;

// #[allow(dead_code)]
pub struct Fs<'a> {
    fio: Fio<'a>,
    dirmap: DirMap,
}

// #[allow(dead_code)]
impl<'a> Fs<'a> {
    pub fn new(device: impl Device + 'a) -> Self {
        let fio = Fio::new(device);
        let mut dirmap = DirMap::new();
        dirmap.insert(1, fio.readroot());
        Fs { fio, dirmap }
    }

    pub fn readdir(&mut self, id: u64) -> &Vec<Finfo> {
        if self.dirmap.get(&id).is_none() {
            let files = self.fio.read_dirents(id as u32);
            self.dirmap.insert(id, files);
        }
        &self.dirmap[&id]
    }

    pub fn lookup(&mut self, parent: u64, name: &str) -> Option<Finfo> {
        for f in self.readdir(parent) {
            if f.name == name {
                return Some(f.clone());
            }
        }
        None
    }
}
