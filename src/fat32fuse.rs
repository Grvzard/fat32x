use std::time::{Duration, UNIX_EPOCH};

use fuser::{FileAttr, FileType, Filesystem, ReplyAttr, Request};
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

impl From<&fat32::fio::File> for FileType {
    fn from(f: &fat32::fio::File) -> Self {
        if f.is_dir {
            Self::Directory
        } else {
            Self::RegularFile
        }
    }
}

impl From<fat32::fio::File> for FileAttr {
    fn from(f: fat32::fio::File) -> Self {
        FileAttr {
            ino: f.fst_clus.into(),
            size: f.size.into(),
            blocks: 0,
            atime: UNIX_EPOCH,
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind: (&f).into(),
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
    ino: 2,
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
        mut parent: u64,
        _name: &std::ffi::OsStr,
        reply: fuser::ReplyEntry,
    ) {
        let name = _name.to_string_lossy();
        println!("lookup `{name}` from `{parent}`");
        if parent == 1 {
            parent = 2;
        }

        if let Some(file) = self.fs.lookup(parent, &name) {
            reply.entry(&TTL, &FileAttr::from(file), 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        println!("getattr ino: {ino}");
        if ino == 1 {
            reply.attr(&TTL, &ROOT_DIR_ATTR)
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &fuser::Request<'_>,
        mut ino: u64,
        _fh: u64,
        _offset: i64,
        mut reply: fuser::ReplyDirectory,
    ) {
        if ino == 1 {
            ino = 2;
        }

        println!("readdir ino: {ino}");
        for (i, f) in self
            .fs
            .readdir(ino)
            .iter()
            .enumerate()
            .skip(_offset as usize)
        {
            println!("{:?}", f);
            if reply.add(f.fst_clus.into(), (i + 1) as i64, f.into(), f.name.clone()) {
                println!("readdir: break;");
                break;
            }
        }
        reply.ok()
    }
}
