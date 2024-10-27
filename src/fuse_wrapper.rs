use std::fs::File;
use std::time::{Duration, UNIX_EPOCH};

use fuser::{FileAttr, FileType, Filesystem, ReplyAttr, ReplyOpen, Request};
use libc::ENOENT;

use crate::fio::{self, Finfo};
use crate::fs;

pub struct FuseW {
    fs: fs::Fs,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum FsType {
    Fat32,
    Exfat,
}

// impl FromStr for FsType {
//     type Err = std::io::Error;
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s {
//             "fat32" => Ok(Self::Fat32),
//             "exfat" => Ok(Self::Exfat),
//             _ => Err(std::io::Error::new(
//                 std::io::ErrorKind::InvalidInput,
//                 "invalid fs type",
//             )),
//         }
//     }
// }

impl FuseW {
    pub fn new(devname: &str, typ: FsType) -> Self {
        let device = File::open(devname).unwrap();
        let fio: Box<dyn fio::Fio> = match typ {
            FsType::Fat32 => Box::new(fio::fat32::Fio::new(device)),
            FsType::Exfat => Box::new(fio::exfat::Fio::new(device)),
        };
        FuseW {
            fs: fs::Fs::new(fio),
        }
    }
}

impl From<&Finfo> for FileType {
    fn from(f: &Finfo) -> Self {
        if f.is_dir {
            Self::Directory
        } else {
            Self::RegularFile
        }
    }
}

impl From<&Finfo> for FileAttr {
    fn from(f: &Finfo) -> Self {
        FileAttr {
            ino: f.id,
            size: f.size,
            blocks: 0,
            atime: f.acc_time,
            mtime: f.wrt_time,
            ctime: f.crt_time, // `imprecise`
            crtime: f.crt_time,
            kind: f.into(),
            perm: 0o755,
            nlink: 2,
            uid: 501,
            gid: 20,
            rdev: 0,
            flags: 0,
            blksize: 512,
        }
    }
}

const TTL: Duration = Duration::from_secs(10);
const ROOT_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

impl Filesystem for FuseW {
    fn lookup(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        _name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let name = _name.to_string_lossy();
        // println!("lookup `{name}` from `{parent}`");

        if let Some(file) = self.fs.lookup(parent, &name) {
            reply.entry(&TTL, &FileAttr::from(file.as_ref()), 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        // println!("getattr ino: {ino}");
        if ino == 1 {
            reply.attr(&TTL, &ROOT_DIR_ATTR)
        } else if let Some(fi) = self.fs.getinfo(ino) {
            // println!("{:?}", fi);
            reply.attr(&TTL, &fi.as_ref().into())
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        ino: u64,
        _fh: u64,
        _offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        println!("readdir ino: {ino}");
        for (i, f) in self
            .fs
            .readdir(ino)
            .iter()
            .enumerate()
            .skip(_offset as usize)
        {
            if reply.add(f.id, (i + 1) as i64, f.as_ref().into(), f.name.clone()) {
                println!("readdir: break;");
                break;
            }
        }
        reply.ok()
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        if self.fs.open(ino) {
            reply.opened(0, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: fuser::ReplyEmpty,
    ) {
        self.fs.close(ino);
        reply.ok();
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: fuser::ReplyData,
    ) {
        if let Some(bytes) = self.fs.read(ino, offset as u32, size) {
            reply.data(&bytes);
        } else {
            reply.error(ENOENT);
        }
    }

    fn opendir(&mut self, _req: &Request<'_>, _ino: u64, _flags: i32, reply: ReplyOpen) {
        if let Some(fi) = self.fs.getinfo(_ino) {
            println!("[fuse] open dir: {}", fi.name);
        }
        reply.opened(0, 0);
    }

    fn releasedir(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        reply: fuser::ReplyEmpty,
    ) {
        if let Some(fi) = self.fs.getinfo(_ino) {
            println!("[fuse] close dir: {}", fi.name);
        }
        reply.ok();
    }
}
