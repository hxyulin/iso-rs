use clap::Parser;
use iso9660_rs::{FormatOptions, directory};
use std::{fs::OpenOptions, io::Write, path::PathBuf};

#[derive(Parser)]
pub struct Args {
    input: PathBuf,
}

fn main() {
    let args = Args::parse();

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
    file.set_len(1024 * 2048 * 2048).unwrap();
    iso9660_rs::IsoImage::format_new(
        &mut file,
        FormatOptions {
            files: vec![
                iso9660_rs::IsoFile::File {
                    name: "test.txt".to_string(),
                    data: vec![b'H'; 1024 * 1024],
                },
                iso9660_rs::IsoFile::Directory {
                    name: "test".to_string(),
                    entries: vec![
                        iso9660_rs::IsoFile::File {
                            name: "test.txt".to_string(),
                            data: vec![b'B'; 1024 * 1024],
                        },
                        iso9660_rs::IsoFile::Directory {
                            name: "test".to_string(),
                            entries: vec![iso9660_rs::IsoFile::File {
                                name: "test.txt".to_string(),
                                data: vec![b'C'; 1024 * 1024],
                            }],
                        },
                    ],
                },
            ],
        },
    )
    .unwrap();
    file.flush().unwrap();
}

fn read(file: &PathBuf) {
    let mut file = OpenOptions::new().read(true).open(file).unwrap();
    let mut iso = iso9660_rs::IsoImage::new(&mut file).unwrap();
    let mut root_dir = iso.root_directory();
    println!("Root Directory: {:#?}", root_dir.entries());
    println!("Path table: {:#?}", iso.path_table().entries());
}
