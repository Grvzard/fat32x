use std::collections::BTreeMap;

use super::fio::{Device, File, Fio};

type DirMap = BTreeMap<u64, Vec<File>>;

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
        dirmap.insert(2, fio.readroot());
        Fs { fio, dirmap }
    }

    pub fn readdir(&mut self, ino: u64) -> &Vec<File> {
        if self.dirmap.get(&ino).is_none() {
            let files = self.fio.read_dirents(ino as u32);
            self.dirmap.insert(ino, files);
        }
        &self.dirmap[&ino]
    }

    pub fn lookup(&mut self, parent: u64, name: &str) -> Option<File> {
        for f in self.readdir(parent) {
            if f.name == name {
                return Some(f.clone());
            }
        }
        None
    }
}
