use clap::Parser;
use iso_rs::FormatOptions;
use std::{fs::OpenOptions, path::PathBuf};

#[derive(Parser)]
pub struct Args {
    input: PathBuf,
}

fn main() {
    let args = Args::parse();

    /*
    let mut file = OpenOptions::new().read(true).open(args.input).unwrap();
    let mut iso = iso_rs::IsoImage::new(&mut file).unwrap();
    dbg!(&iso);
    println!("Files: {:#?}", iso.root_directory().entries().unwrap());
    println!("Paths: {:#?}", iso.path_table().entries().unwrap());
*/
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(args.input)
        .unwrap();
    file.set_len(1024 * 2048).unwrap();
    iso_rs::IsoImage::format_new(
        &mut file,
        FormatOptions {
            files: vec![iso_rs::IsoFile::File {
                name: "test.txt".to_string(),
                data: vec![b'H'; 1024 * 1024],
            }],
        },
    )
    .unwrap();
}
