#![allow(clippy::unused_io_amount)]

extern crate byteorder;
extern crate clap;
extern crate png;

use byteorder::{BigEndian, WriteBytesExt};
use clap::{App, Arg};
use png::{BitDepth, ColorType, Decoder};
use std::collections::HashSet;
use std::fs::{read, write};
use std::io::{Cursor, Write};

type Sprite = Vec<u8>;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

fn sprite_from_chunk_rgb(buffer: &[u8], col: usize, row: usize, width: usize) -> Sprite {
    let mut bytes = vec![0; 16 * 16 * 3];
    for ty in 0..16 {
        for tx in 0..16 {
            let to = ty * 16 * 3 + tx * 3;
            let from = (row * 16 * 3 + ty * 3) * width + col * 16 * 3 + tx * 3;
            bytes[to] = buffer[from];
            bytes[to + 1] = buffer[from + 1];
            bytes[to + 2] = buffer[from + 2];
        }
    }
    bytes
}

fn sprite_from_chunk_rgba(buffer: &[u8], col: usize, row: usize, width: usize) -> Sprite {
    let mut bytes = vec![0; 16 * 16 * 3];
    for ty in 0..16 {
        for tx in 0..16 {
            let to = ty * 16 * 3 + tx * 3;
            let from = (row * 16 * 4 + ty * 4) * width + col * 16 * 4 + tx * 4;
            bytes[to] = buffer[from];
            bytes[to + 1] = buffer[from + 1];
            bytes[to + 2] = buffer[from + 2];
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
) -> Sprite {
    unsafe {
        let mut bytes = vec![0; 16 * 16 * 3];
        let colors = palette.as_ptr() as *const Color;
        for ty in 0..16 {
            for tx in 0..16 {
                let to = ty * 16 * 3 + tx * 3;
                let from = (row * 16 + ty) * width + col * 16 + tx;
                let color = *colors.add(buffer[from] as usize);
                bytes[to] = color.r;
                bytes[to + 1] = color.g;
                bytes[to + 2] = color.b;
            }
        }
        bytes
    }
}

fn collect_colors(sprites: &[Sprite]) -> Vec<Color> {
    let mut colors = HashSet::with_capacity(16 * 16 * sprites.len());
    for sprite in sprites {
        unsafe {
            let source = sprite.as_ptr() as *const Color;
            for i in 0..(sprite.len() / 3) {
                colors.insert(*source.add(i));
            }
        }
    }
    colors.into_iter().collect()
}

fn compress_sprite(sprites: &[Sprite], colors: &[Color]) -> Vec<Sprite> {
    sprites
        .iter()
        .map(|sprite| unsafe {
            let mut result = vec![0; 16 * 16];
            let source = sprite.as_ptr() as *const Color;
            #[allow(clippy::needless_range_loop)]
            for i in 0..(sprite.len() / 3) {
                result[i] = colors.iter().position(|c| *source.add(i) == *c).unwrap() as u8;
            }
            result
        })
        .collect()
}

fn main() {
    let matches = App::new("Cartridge Packager CLI")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("assembly")
                .short("a")
                .long("assembly")
                .value_name("FILE")
                .help("Path to Kaiju assembly binary (*.kjb)")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("sprites")
                .short("s")
                .long("sprites")
                .value_name("FILE")
                .help("Spritesheet to include (*.png)")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("data")
                .short("d")
                .long("data")
                .value_name("FILE")
                .help("Raw bytes to include (*.*)")
                .required(false)
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
        .arg(
            Arg::with_name("graphics")
                .short("g")
                .long("graphics")
                .value_name("FILE")
                .help("Graphics mode (320x240; 160x128)")
                .takes_value(true)
                .default_value("320x240"),
        )
        .get_matches();

    let assembly = matches.value_of("assembly").unwrap();
    let sprites = matches.value_of("sprites").unwrap();
    let output = matches.value_of("output").unwrap();

    let assembly = match read(assembly) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{}: Could not read assembly: {:?}", assembly, err);
            ::std::process::exit(1);
        }
    };
    let (sprites, colors) = match read(sprites) {
        Ok(data) => {
            let path = &sprites;
            let (info, mut reader) = match Decoder::new(Cursor::new(data)).read_info() {
                Ok(r) => r,
                Err(err) => {
                    eprintln!("{}: Could not decode sprites: {:?}", sprites, err);
                    ::std::process::exit(1);
                }
            };
            let palette = reader.info().palette.clone();
            let mut buffer = vec![0; info.buffer_size()];
            if let Err(err) = reader.next_frame(&mut buffer) {
                eprintln!("{}: Could not read spritesheet frame: {:?}", sprites, err);
                ::std::process::exit(1);
            }
            if info.width % 16 != 0 {
                eprintln!(
                    "{}: Spritesheet does not have width divisible by 16",
                    sprites
                );
                ::std::process::exit(1);
            }
            if info.height % 16 != 0 {
                eprintln!(
                    "{}: Spritesheet does not have height divisible by 16",
                    sprites
                );
                ::std::process::exit(1);
            }
            if info.bit_depth != BitDepth::Eight {
                eprintln!("{}: Spritesheet does not have 8-bit color depth", sprites);
                ::std::process::exit(1);
            }
            match info.color_type {
                ColorType::RGB | ColorType::RGBA | ColorType::Indexed => {}
                _ => {
                    eprintln!(
                        "{}: Spritesheet is neither RGB, RGBA or indexed type",
                        sprites
                    );
                    ::std::process::exit(1);
                }
            }
            let cols = info.width as usize / 16;
            let rows = info.height as usize / 16;
            if cols * rows > 256 {
                eprintln!("{}: Spritesheet have more than 256 sprites", sprites);
                ::std::process::exit(1);
            }
            let mut sprites = Vec::with_capacity(cols * rows);
            for row in 0..rows {
                for col in 0..cols {
                    sprites.push(match info.color_type {
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
            let colors = collect_colors(&sprites);
            if colors.len() > 256 {
                eprintln!(
                    "{}: Spritesheet has more than 256 colors: {}",
                    path,
                    colors.len()
                );
                ::std::process::exit(1);
            }
            let sprites = compress_sprite(&sprites, &colors);
            (sprites, colors)
        }
        Err(err) => {
            eprintln!("{}: Could not read spritesheet: {:?}", sprites, err);
            ::std::process::exit(1);
        }
    };

    let mut stream = Cursor::new(vec![]);
    stream.write(&[67u8, 65u8, 82u8, 84u8]).unwrap();
    if let Some(mode) = matches.value_of("graphics") {
        match mode {
            "320x240" => stream.write_u8(0).unwrap(),
            "160x128" => stream.write_u8(1).unwrap(),
            _ => {
                eprintln!("Unsupported graphics mode: {}", mode);
                ::std::process::exit(1);
            }
        }
    }
    stream.write_u8(colors.len() as u8).unwrap();
    for color in colors {
        stream.write_u8(color.r).unwrap();
        stream.write_u8(color.g).unwrap();
        stream.write_u8(color.b).unwrap();
    }
    stream.write_u8(sprites.len() as u8).unwrap();
    for sprite in sprites {
        stream.write(&sprite).unwrap();
    }
    if let Some(data) = matches.value_of("data") {
        let path = &data;
        match read(data) {
            Ok(data) => {
                if data.len() > 8 * 1024 {
                    eprintln!("{}: Data is bigger than 8kb: {:?}", path, data.len());
                    ::std::process::exit(1);
                }
                stream.write_u16::<BigEndian>(data.len() as u16).unwrap();
                stream.write(&data).unwrap();
            }
            Err(err) => {
                eprintln!("{}: Could not read data: {:?}", data, err);
                ::std::process::exit(1);
            }
        }
    } else {
        stream.write_u16::<BigEndian>(0).unwrap();
    }
    stream
        .write_u64::<BigEndian>(assembly.len() as u64)
        .unwrap();
    stream.write(&assembly).unwrap();
    if let Err(err) = write(output, stream.into_inner()) {
        eprintln!("{}: Could not write cartridge: {:?}", output, err);
        ::std::process::exit(1);
    }
}
