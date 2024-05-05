use std::io::{Read, Seek, SeekFrom};

mod spec {
    use scroll::{self, Pread, LE};

    #[derive(Debug)]
    pub struct BootSec {
        pub jmp_boot: [u8; 3],     // `unused`
        file_system_name: [u8; 8], // check only, "EXFAT   "
        must_be_zero: [u8; 53],    // check only, zeros
        pub partition_offset: u64,
        pub volumn_length: u64,
        pub fat_offset: u32,
        pub fat_length: u32,
        pub cluster_heap_offset: u32,
        pub cluster_count: u32,
        pub first_cluster_of_root_directory: u32,
        pub volumn_serial_number: u32, // `unused`
        pub file_system_revision: u16, // `unused`
        pub volumn_flags: u16,         // `unused`
        pub bytes_per_sector_shift: u8,
        pub sectors_per_cluster_shift: u8,
        pub number_of_fats: u8,
        pub drive_select: u8,     // `unused`
        pub percent_in_use: u8,   // `unused`
        pub reserved: [u8; 7],    // `unused`
        pub boot_code: [u8; 390], // `unused`
        boot_signature: u16,      // check only, "0xAA55"
    }

    #[allow(dead_code)]
    impl BootSec {
        pub fn new(buf: &[u8; 512]) -> Result<Self, scroll::Error> {
            Ok(BootSec {
                jmp_boot: buf.pread_with(0, LE)?,
                file_system_name: buf.pread_with(3, LE)?,
                must_be_zero: buf.pread_with(11, LE)?,
                partition_offset: buf.pread_with(64, LE)?,
                volumn_length: buf.pread_with(72, LE)?,
                fat_offset: buf.pread_with(80, LE)?,
                fat_length: buf.pread_with(84, LE)?,
                cluster_heap_offset: buf.pread_with(88, LE)?,
                cluster_count: buf.pread_with(92, LE)?,
                first_cluster_of_root_directory: buf.pread_with(96, LE)?,
                volumn_serial_number: buf.pread_with(100, LE)?,
                file_system_revision: buf.pread_with(104, LE)?,
                volumn_flags: buf.pread_with(106, LE)?,
                bytes_per_sector_shift: buf.pread_with(108, LE)?,
                sectors_per_cluster_shift: buf.pread_with(109, LE)?,
                number_of_fats: buf.pread_with(110, LE)?,
                drive_select: buf.pread_with(111, LE)?,
                percent_in_use: buf.pread_with(112, LE)?,
                reserved: buf.pread_with(113, LE)?,
                boot_code: buf.pread_with(120, LE)?,
                boot_signature: buf.pread_with(510, LE)?,
            })
        }

        pub fn is_valid(&self) -> bool {
            self.file_system_name == "EXFAT   ".as_bytes()
                && self.must_be_zero.iter().all(|&b| b == 0)
                && self.boot_signature == 0xAA55
        }
    }
}

use spec::BootSec;

const SEC_SZ: usize = 512;
type Sec = [u8; SEC_SZ];

#[allow(dead_code)]
pub struct Fio<D: Seek + Read> {
    device: D,
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
        Fio { device, bootsec }
    }
}
