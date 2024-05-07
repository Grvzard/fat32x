use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct Finfo {
    pub id: u64, // a unique id consists of entry's clus_no and offset
    pub name: String,
    pub is_rdonly: bool, // `unused`, especially in FAT fs
    pub is_hidden: bool, // `unused`, especially in FAT fs
    pub is_system: bool, // `unused`, especially in FAT fs
    pub is_dir: bool,
    pub size32: u32, // used in Fat32
    pub size: u64,
    pub fst_clus: u32, // implementation specific field
    pub crt_time: SystemTime,
    pub wrt_time: SystemTime,
    pub acc_time: SystemTime,
    // pub ctime: SystemTime, // last change time
}

pub trait Fio {
    fn list_dir(&mut self, no: u32) -> Vec<Finfo>;
    fn list_root(&mut self) -> Vec<Finfo>;
    fn read_file(&mut self, fi: &Finfo, offset: u32, size: u32) -> Vec<u8>;
}
