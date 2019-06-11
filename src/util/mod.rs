use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

pub fn read_as_byte_buffer(path_str: &str) -> Result<Vec<u8>, io::Error> {
    let path = Path::new(path_str);
    let mut file = File::open(path)?;

    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

pub fn write_bytes_to_file(path_str: &str, buffer: &Vec<u8>) -> Result<(), io::Error> {
    let path = Path::new(path_str);
    let mut file = File::create(path)?;

    file.write_all(buffer)?;

    Ok(())
}
