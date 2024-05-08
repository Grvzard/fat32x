mod device;
mod exfat;
mod fat32;
mod fat32fuse;
mod fio;
mod fs;
mod mbr;

use std::{
    fs::File,
    io::{Read, Write},
};

use clap::{builder::PossibleValue, Parser, Subcommand};
use fuser::MountOption;

use fat32fuse::{FsType, FuseW};
use mbr::Mbr;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Mount {
        device: String,
        mount_point: String,
        #[arg(short, long, value_enum)]
        r#type: FsType,
    },
    Fat32 {
        device: String,
        #[arg(short, long, group = "instr")]
        info: bool,
        #[arg(
            short,
            long,
            group = "instr",
            default_value_t = 0,
            value_name = "ClusNo"
        )]
        read_clus: u32,
    },
    Exfat {
        device: String,
        #[arg(short, long, group = "instr")]
        info: bool,
        #[arg(
            short,
            long,
            group = "instr",
            default_value_t = 0,
            value_name = "ClusNo"
        )]
        read_clus: u32,
        #[arg(long, group = "instr", default_value_t = 0, value_name = "ClusNo")]
        read_dirents: u32,
    },
    Mbr {
        device: String,
    },
}

impl clap::ValueEnum for FsType {
    fn value_variants<'a>() -> &'a [Self] {
        &[FsType::Fat32, FsType::Exfat]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match *self {
            FsType::Fat32 => Some(PossibleValue::new("fat32")),
            FsType::Exfat => Some(PossibleValue::new("exfat")),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Mount {
            device,
            mount_point,
            r#type,
        } => {
            let opts = vec![
                MountOption::AllowOther,
                MountOption::AutoUnmount,
                MountOption::RO,
            ];
            match fuser::mount2(FuseW::new(device, r#type.clone()), mount_point, &opts) {
                Ok(()) => (),
                Err(e) => {
                    println!("{}", e);
                }
            };
        }
        Commands::Fat32 {
            device,
            info,
            read_clus,
        } => {
            let mut fio = fat32::fio::Fio::new(File::open(device).unwrap());
            if *info {
                println!("{:?}", fio.bootsec)
            } else if *read_clus != 0 {
                let clus = fio.read_clus(*read_clus);
                std::io::stdout().write_all(&clus).unwrap();
            }
        }
        Commands::Exfat {
            device,
            info,
            read_clus,
            read_dirents,
        } => {
            let file = File::open(device).expect("device can't be opened");
            let mut fio = exfat::Fio::new(file);
            if *info {
                println!("{:?}", fio.bootsec)
            } else if *read_clus != 0 {
                let clus = fio.read_clus(*read_clus);
                std::io::stdout().write_all(&clus).unwrap();
            } else if *read_dirents != 0 {
                let ents = fio.read_dirents(*read_dirents);
                println!("{:#?}", ents);
            }
        }
        Commands::Mbr { device } => {
            let mut file = File::open(device).expect("device can't be opened");
            let mut buf = [0u8; 512];
            file.read_exact(&mut buf).unwrap();
            let mbr = Mbr::new(&buf).unwrap();
            println!("{:X?}", mbr);
        }
    }
}
