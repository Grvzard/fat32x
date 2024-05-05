mod fat32;
mod fat32fuse;

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
            match fuser::mount2(fat32fuse::Fat32Fuse::new(&device), mount_point, &opts) {
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
            let fio = fat32::fio::Fio::new(fat32::impls::BlkDevice::new(device));
            if *info {
                println!("{:?}", fio.bootsec)
            } else if *read_clus != 0 {
                let clus = fio.read_clus(*read_clus);
                for byte in clus {
                    print!("{}", byte as char);
                }
            }
        }
    }
}
