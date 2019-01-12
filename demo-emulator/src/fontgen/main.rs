#![allow(clippy::unused_io_amount)]

extern crate clap;
extern crate png;

use clap::{App, Arg};
use png::{BitDepth, ColorType, Decoder};
use std::fs::{read, write};
use std::io::{Cursor, Write};

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

fn sprite_from_chunk_rgb(buffer: &[u8], col: usize, row: usize, width: usize) -> Vec<u8> {
    let mut bytes = vec![0; 8 * 8];
    for ty in 0..8 {
        for tx in 0..8 {
            let to = ty * 8 + tx;
            let from = (row * 8 * 3 + ty * 3) * width + col * 8 * 3 + tx * 3;
            bytes[to] = if buffer[from] > 0 { 1 } else { 0 };
        }
    }
    bytes
}

fn sprite_from_chunk_rgba(buffer: &[u8], col: usize, row: usize, width: usize) -> Vec<u8> {
    let mut bytes = vec![0; 8 * 8];
    for ty in 0..8 {
        for tx in 0..8 {
            let to = ty * 8 + tx;
            let from = (row * 8 * 4 + ty * 4) * width + col * 8 * 4 + tx * 4;
            bytes[to] = if buffer[from] > 0 { 1 } else { 0 };
        }
    }
    bytes
}

fn sprite_from_chunk_indexed(
    buffer: &[u8],
    col: usize,
    row: usize,
    width: usize,
    palette: &[u8],
) -> Vec<u8> {
    unsafe {
        let mut bytes = vec![0; 8 * 8];
        let colors = palette.as_ptr() as *const Color;
        for ty in 0..8 {
            for tx in 0..8 {
                let to = ty * 8 + tx;
                let from = (row * 8 + ty) * width + col * 8 + tx;
                let color = *colors.add(buffer[from] as usize);
                bytes[to] = if color.r > 0 { 1 } else { 0 };
            }
        }
        bytes
    }
}

fn main() {
    let matches = App::new("Font Data Generator CLI")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Path to PNG with 8x8 font characters (*.png)")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .value_name("FILE")
                .help("Cardridge package (*.cart)")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let input = matches.value_of("input").unwrap();
    let output = matches.value_of("output").unwrap();

    let characters = match read(input) {
        Ok(data) => {
            let (info, mut reader) = match Decoder::new(Cursor::new(data)).read_info() {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("{}: Could not decode font: {:?}", input, err);
                    ::std::process::exit(1);
                }
            };
            let palette = reader.info().palette.clone();
            let mut buffer = vec![0; info.buffer_size()];
            if let Err(err) = reader.next_frame(&mut buffer) {
                eprintln!("{}: Could not read font frame: {:?}", input, err);
                ::std::process::exit(1);
            }
            if info.width % 8 != 0 {
                eprintln!("{}: Font does not have width divisible by 8", input);
                ::std::process::exit(1);
            }
            if info.height % 8 != 0 {
                eprintln!("{}: Font does not have height divisible by 8", input);
                ::std::process::exit(1);
            }
            if info.bit_depth != BitDepth::Eight {
                eprintln!("{}: Font does not have 8-bit color depth", input);
                ::std::process::exit(1);
            }
            match info.color_type {
                ColorType::RGB | ColorType::RGBA | ColorType::Indexed => {}
                _ => {
                    eprintln!("{}: Input is neither RGB, RGBA or indexed type", input);
                    ::std::process::exit(1);
                }
            }
            let cols = info.width as usize / 8;
            let rows = info.height as usize / 8;
            if cols * rows > 256 {
                eprintln!("{}: Font have more than 256 characters", input);
                ::std::process::exit(1);
            }
            let mut chars = Vec::with_capacity(cols * rows);
            for row in 0..rows {
                for col in 0..cols {
                    chars.push(match info.color_type {
                        ColorType::RGB => {
                            sprite_from_chunk_rgb(&buffer, col, row, info.width as usize)
                        }
                        ColorType::RGBA => {
                            sprite_from_chunk_rgba(&buffer, col, row, info.width as usize)
                        }
                        ColorType::Indexed => sprite_from_chunk_indexed(
                            &buffer,
                            col,
                            row,
                            info.width as usize,
                            &palette.clone().unwrap(),
                        ),
                        _ => unreachable!(),
                    });
                }
            }
            chars
        }
        Err(err) => {
            eprintln!("{}: Could not read font: {:?}", input, err);
            ::std::process::exit(1);
        }
    };

    let mut stream = Cursor::new(vec![]);
    for c in characters {
        stream.write(&c).unwrap();
    }
    if let Err(err) = write(output, stream.into_inner()) {
        eprintln!("{}: Could not write fonts data: {:?}", output, err);
        ::std::process::exit(1);
    }
}
