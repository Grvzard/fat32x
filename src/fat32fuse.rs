use std::time::{Duration, UNIX_EPOCH};

use fuser::{FileAttr, FileType, Filesystem, ReplyAttr, ReplyOpen, Request};
use libc::ENOENT;

use crate::fat32;

pub struct Fat32Fuse<'a> {
    fs: fat32::fs::Fs<'a>,
}

impl<'a> Fat32Fuse<'a> {
    pub fn new(devname: &str) -> Self {
        let device = fat32::impls::BlkDevice::new(devname);
        Fat32Fuse {
            fs: fat32::fs::Fs::new(device),
        }
    }
}

impl From<&fat32::fio::Finfo> for FileType {
    fn from(f: &fat32::fio::Finfo) -> Self {
        if f.is_dir {
            Self::Directory
        } else {
            Self::RegularFile
        }
    }
}

impl From<&fat32::fio::Finfo> for FileAttr {
    fn from(f: &fat32::fio::Finfo) -> Self {
        FileAttr {
            ino: f.id,
            size: f.size.into(),
            blocks: 0,
            atime: f.wrt_time, // `imprecise`
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

impl<'a> Filesystem for Fat32Fuse<'a> {
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
