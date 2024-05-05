mod device;
mod exfat;
mod fat32;
mod fat32fuse;
mod fs;

use std::{fs::File, io::Write};

use clap::{Parser, Subcommand};
use fuser::MountOption;

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
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Mount {
            device,
            mount_point,
        } => {
            let opts = vec![
                MountOption::AllowOther,
                MountOption::AutoUnmount,
                MountOption::RO,
            ];
            match fuser::mount2(fat32fuse::Fat32Fuse::new(device), mount_point, &opts) {
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
        Commands::Exfat { device, info } => {
            let file = File::open(device).expect("device can't be opened");
            let fio = exfat::Fio::new(file);
            if *info {
                println!("{:?}", fio.bootsec)
            }
        }
    }
}
