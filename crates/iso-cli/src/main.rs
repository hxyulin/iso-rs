use clap::Parser;
use iso9660_rs::{file::FileInput, ElToritoOptions, FormatOptions};
use std::{fs::OpenOptions, io::Write, path::PathBuf};

#[derive(Parser)]
pub struct Args {
    input: PathBuf,
}

fn main() {
    let args = Args::parse();
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Trace)
        .init()
        .unwrap();

    write(&args.input);
    read(&args.input);
}

fn write(file: &PathBuf) {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(file)
        .unwrap();
    // We zero the file out to make sure we don't have any old data
    file.set_len(0).unwrap();
    file.sync_data().unwrap();
    file.set_len(128 * 2048 * 2048).unwrap();
    iso9660_rs::IsoImage::format_new(
        &mut file,
        FormatOptions {
            files: FileInput::from_fs(concat!(env!("CARGO_MANIFEST_DIR"), "/isoroot").into()).unwrap(),
            protective_mbr: true,
            el_torito: Some(ElToritoOptions {
                load_size: 4,
                boot_image_path: "limine-bios-cd.bin".to_string(),
                boot_info_table: true,
            })
        },
    )
    .unwrap();
    file.flush().unwrap();
}

fn read(file: &PathBuf) {
    let mut file = OpenOptions::new().read(true).open(file).unwrap();
    let mut iso = iso9660_rs::IsoImage::new(&mut file).unwrap();
    let mut root_dir = iso.root_directory();
    //println!("Root Directory: {:#?}", root_dir.entries());
    //println!("Path table: {:#?}", iso.path_table().entries());
}
