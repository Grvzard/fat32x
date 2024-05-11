// References:
// [1] https://www.nongnu.org/ext2-doc/ext2.html

#![allow(dead_code)]

mod spec {
    use scroll::{Pread, LE};

    #[derive(Debug)]
    pub struct Sblk {
        inodes_cnt: u32,
        blocks_cnt: u32,
        r_blocks_cnt: u32,
        free_blocks_cnt: u32,
        free_inodes_cnt: u32,
        first_data_block: u32,
        log2_block_size: u32, // in KBytes
        log2_frag_size: u32,  // in KBytes
        blocks_per_group: u32,
        frags_per_group: u32,
        inodes_per_group: u32,
        mtime: u32,           // `unused`
        wtime: u32,           // `unused`
        mnt_cnt: u16,         // `unused`
        max_mnt_cnt: u16,     // `unused`
        magic: u16,           // check only
        state: u16,           // check only
        errors: u16,          // `unused`
        minor_rev_level: u16, // `unused`
        lastcheck: u32,       // `unused`
        checkinterval: u32,   // `unused`
        creator_os: u32,      // `unused`
        rev_level: u32,       // check only
        def_resuid: u16,      // `unused`
        def_resgid: u16,      // `unused`
        // 84..=204 EXT2_DYNAMIC_REV
        first_ino: u32,
        inode_size: u16,
        block_group_nr: u16,    // `unused`
        feature_compat: u32,    // `unused`
        feature_incompat: u32,  // check only
        feature_ro_compat: u32, // `unused`
        uuid: [u8; 16],         // `unused`
        volume_name: [u8; 16],  // `unused`
        last_mounted: [u8; 64], // `unused`
        algo_bitmap: u32,       // `unused`
        // 204..=205 Performance Hints
        prealloc_blocks: u8,
        realloc_dir_blocks: u8,
        // 208..=236 Journaling Support
        journal_uuid: [u8; 16], // `unused`
        journal_inum: u32,      // `unused`
        journal_dev: u32,       // `unused`
        last_orphan: u32,       // `unused`
        // 236..=252 Directory Indexing Support
        hash_seed: [u32; 4],
        def_hash_version: u8,
        // 256..=263 Other options
        default_mount_options: u32, // `unknown`
        first_meta_bg: u32,         // `unknown`
    }

    impl Sblk {
        const EXT2_SUPER_MAGIC: u16 = 0xEF53;
        const EXT2_GOOD_OLD_INODE_SIZE: u16 = 128;
        const EXT2_GOOD_OLD_FIRST_INO: u32 = 11;
        const EXT2_FEATURE_INCOMPAT_COMPRESSION: u32 = 0x01;
        const EXT2_FEATURE_INCOMPAT_FILETYPE: u32 = 0x02;
        const EXT3_FEATURE_INCOMPAT_RECOVER: u32 = 0x04;
        const EXT3_FEATURE_INCOMPAT_JOURNAL_DEV: u32 = 0x08;
        const EXT2_FEATURE_INCOMPAT_META_BG: u32 = 0x10;
        const EXT2_VALID_FS: u16 = 1;
        const EXT2_ERROR_FS: u16 = 2;

        pub fn new(buf: &[u8]) -> Result<Self, scroll::Error> {
            assert!(buf.len() >= 1024);
            Ok(Sblk {
                inodes_cnt: buf.pread_with(0, LE)?,
                blocks_cnt: buf.pread_with(4, LE)?,
                r_blocks_cnt: buf.pread_with(8, LE)?,
                free_blocks_cnt: buf.pread_with(12, LE)?,
                free_inodes_cnt: buf.pread_with(16, LE)?,
                first_data_block: buf.pread_with(20, LE)?,
                log2_block_size: buf.pread_with(24, LE)?,
                log2_frag_size: buf.pread_with(28, LE)?,
                blocks_per_group: buf.pread_with(32, LE)?,
                frags_per_group: buf.pread_with(36, LE)?,
                inodes_per_group: buf.pread_with(40, LE)?,
                mtime: buf.pread_with(44, LE)?,
                wtime: buf.pread_with(48, LE)?,
                mnt_cnt: buf.pread_with(52, LE)?,
                max_mnt_cnt: buf.pread_with(54, LE)?,
                magic: buf.pread_with(56, LE)?,
                state: buf.pread_with(58, LE)?,
                errors: buf.pread_with(60, LE)?,
                minor_rev_level: buf.pread_with(62, LE)?,
                lastcheck: buf.pread_with(64, LE)?,
                checkinterval: buf.pread_with(68, LE)?,
                creator_os: buf.pread_with(72, LE)?,
                rev_level: buf.pread_with(76, LE)?,
                def_resuid: buf.pread_with(80, LE)?,
                def_resgid: buf.pread_with(82, LE)?,
                // 84..=204 EXT2_DYNAMIC_REV
                first_ino: buf.pread_with(84, LE)?,
                inode_size: buf.pread_with(88, LE)?,
                block_group_nr: buf.pread_with(90, LE)?,
                feature_compat: buf.pread_with(92, LE)?,
                feature_incompat: buf.pread_with(96, LE)?,
                feature_ro_compat: buf.pread_with(100, LE)?,
                uuid: buf.pread_with(104, LE)?,
                volume_name: buf.pread_with(120, LE)?,
                last_mounted: buf.pread_with(136, LE)?,
                algo_bitmap: buf.pread_with(200, LE)?,
                // 204..=205 Performance Hints
                prealloc_blocks: buf.pread_with(204, LE)?,
                realloc_dir_blocks: buf.pread_with(205, LE)?,
                // 208..=236 Journaling Support
                journal_uuid: buf.pread_with(208, LE)?,
                journal_inum: buf.pread_with(224, LE)?,
                journal_dev: buf.pread_with(228, LE)?,
                last_orphan: buf.pread_with(232, LE)?,
                // 236..=252 Directory Indexing Support
                hash_seed: buf.pread_with(236, LE)?,
                def_hash_version: buf.pread_with(252, LE)?,
                // 256..=263 Other options
                default_mount_options: buf.pread_with(256, LE)?,
                first_meta_bg: buf.pread_with(260, LE)?,
            })
        }

        pub fn is_valid(&self) -> bool {
            self.magic == Self::EXT2_SUPER_MAGIC
                && self.state == Self::EXT2_VALID_FS
                && self.feature_incompat & Self::EXT2_FEATURE_INCOMPAT_COMPRESSION == 0
                && self.feature_incompat & Self::EXT3_FEATURE_INCOMPAT_RECOVER == 0
                && self.feature_incompat & Self::EXT3_FEATURE_INCOMPAT_JOURNAL_DEV == 0
                && self.feature_incompat & Self::EXT2_FEATURE_INCOMPAT_META_BG == 0
        }

        #[inline]
        pub fn blk_sz(&self) -> u32 {
            1 << self.log2_block_size << 10
        }

        pub fn is_rev0(&self) -> bool {
            self.rev_level == 0
        }

        pub fn inode_sz(&self) -> u16 {
            if self.is_rev0() {
                Self::EXT2_GOOD_OLD_INODE_SIZE
            } else {
                self.inode_size
            }
        }

        pub fn first_ino(&self) -> u32 {
            if self.is_rev0() {
                Self::EXT2_GOOD_OLD_FIRST_INO
            } else {
                self.first_ino
            }
        }

        pub fn groups_cnt(&self) -> u32 {
            if self.blocks_cnt % self.blocks_per_group != 0 {
                self.blocks_cnt / self.blocks_per_group + 1
            } else {
                self.blocks_cnt / self.blocks_per_group
            }
        }
    }
}

use std::io::{Read, Seek, SeekFrom};

use spec::Sblk;

pub struct Fio<D: Seek + Read> {
    blk_sz: u32,
    bgp_per_block: u32,
    device: D,
    pub sblk: Sblk,
}

impl<D: Seek + Read> Fio<D> {
    pub fn new(mut device: D) -> Self {
        let mut buf = [0u8; 1024];

        device.seek(SeekFrom::Start(1024)).unwrap();
        device.read_exact(&mut buf).unwrap();
        let sblk = Sblk::new(&buf).unwrap();
        assert!(sblk.is_valid());

        Fio {
            blk_sz: sblk.blk_sz(),
            bgp_per_block: sblk.blk_sz() / 32,
            device,
            sblk,
        }
    }

    fn read_block(&mut self, blk_no: u32) -> Vec<u8> {
        let mut buf = vec![0u8; self.blk_sz as usize];
        self.device
            .seek(SeekFrom::Start((blk_no * self.blk_sz) as u64))
            .unwrap();
        self.device.read_exact(&mut buf).unwrap();
        buf
    }
}
