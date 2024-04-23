mod fat32;
mod fat32fuse;

use clap::Parser;
use fuser::MountOption;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    device: String,
    mount_point: String,
}

fn main() {
    let args = Args::parse();
    let opts = vec![
        MountOption::AllowOther,
        MountOption::AutoUnmount,
        MountOption::RO,
    ];
    match fuser::mount2(
        fat32fuse::Fat32Fuse::new(&args.device),
        args.mount_point,
        &opts,
    ) {
        Ok(()) => (),
        Err(e) => {
            println!("{}", e);
        }
    };
}
