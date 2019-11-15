#![allow(clippy::cast_ref_to_mut)]

extern crate byteorder;
extern crate clap;
extern crate kaiju_core as core;
extern crate kaiju_vm_core as vm_core;
extern crate minifb;
#[macro_use]
extern crate lazy_static;
extern crate rayon;

mod cartridge;
mod processor;
mod render;

use crate::cartridge::*;
use crate::processor::*;
use crate::render::*;
use clap::{App, Arg};
use minifb::{InputCallback, Key, Scale, Window, WindowOptions};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::read;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum MapKey {
    Single(String),
    Range(String, String),
}

struct CharsPressed {}

impl InputCallback for CharsPressed {
    fn add_char(&mut self, uni_char: u32) {
        Processor::with_mut(|p| p.chars_pressed.push(uni_char as u8));
    }
}

fn main() {
    let matches = App::new("Console Emulator")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("cartridge")
                .short("c")
                .long("cartridge")
                .value_name("FILE")
                .help("Path to console cartridge image (*.cart)")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("scale")
                .short("s")
                .long("scale")
                .value_name("INTEGER")
                .help("Pixel scale (1, 2, 4, 8)")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("save")
                .long("save")
                .value_name("FILE")
                .help("Path to file used to store and load game save (*.dat)")
                .required(false)
                .takes_value(true),
        )
        .get_matches();

    let (font, chars) = {
        let font: Vec<u8> = include_bytes!("../../res/characters.dat").to_vec();
        let lines = include_str!("../../res/characters_map.txt").lines();
        let size = 8 * 8;
        let count = font.len() / size;
        let mut f = Vec::with_capacity(count);
        for i in 0..count {
            f.push(font[(i * size)..((i + 1) * size)].to_vec());
        }
        let mut m = HashMap::new();
        for line in lines {
            let parts = line.split(' ').collect::<Vec<&str>>();
            let keys = parts[0].split(':').collect::<Vec<&str>>();
            let value = parts[1];
            if keys.len() > 1 {
                m.insert(
                    MapKey::Range(keys[0].to_owned(), keys[1].to_owned()),
                    value.to_owned(),
                );
            } else {
                m.insert(MapKey::Single(keys[0].to_owned()), value.to_owned());
            }
        }
        let def = if let Some(v) = m.get(&MapKey::Single("und".to_string())) {
            v.parse().unwrap()
        } else {
            0u8
        };
        let car = if let Some(v) = m.get(&MapKey::Single("car".to_string())) {
            v.parse().unwrap()
        } else {
            0u8
        };
        let mut chars = vec![def; 256];
        for (k, v) in m {
            let mut v = v.parse::<usize>().unwrap();
            match k {
                MapKey::Single(k) => {
                    if k != "und" && k != "car" {
                        chars[k.parse::<usize>().unwrap()] = v as u8;
                    }
                }
                MapKey::Range(ks, ke) => {
                    for c in chars
                        .iter_mut()
                        .take(ke.parse::<usize>().unwrap() + 1)
                        .skip(ks.parse::<usize>().unwrap())
                    {
                        *c = v as u8;
                        v += 1;
                    }
                }
            }
        }
        chars[255] = car;
        (f, chars)
    };
    let cartridge = matches.value_of("cartridge").unwrap();
    let scale = match matches.value_of("scale").unwrap_or("1") {
        "2" => Scale::X2,
        "4" => Scale::X4,
        "8" => Scale::X8,
        _ => Scale::X1,
    };
    let (sprites, data, mut vm, (width, height)) = {
        let buffer = read(cartridge).unwrap_or_else(|e| panic!("{}", e));
        read_cartridge(&buffer).unwrap_or_else(|e| panic!(e.message))
    };
    vm.start("main").unwrap_or_else(|e| panic!(e.message));
    if !data.is_empty() {
        let v = vm
            .state_mut()
            .alloc_memory_value(data.len())
            .unwrap_or_else(|e| panic!(e.message));
        vm.state_mut()
            .store_bytes(v.address, &data)
            .unwrap_or_else(|e| panic!(e.message));
        Processor::with_mut(|p| p.data_block = v);
    }
    let tcols = 2 * width / 16;
    let trows = 2 * height / 16;
    Processor::with_mut(|p| {
        p.tcols = tcols;
        p.trows = trows;
        p.sprites = sprites.clone();
        p.tiles = vec![0; tcols * trows];
        if let Some(path) = matches.value_of("save") {
            p.save_file = path.to_owned();
        }
        p.font = font.clone();
        p.chars = chars.clone();
        p.text_cols = tcols;
        p.text_rows = trows;
        p.text_buffer = vec![0; tcols * trows];
    });
    drop(sprites);
    drop(font);
    drop(chars);
    let mut buffer: Vec<u32> = vec![0; width * height];
    let mut window = Window::new(
        &format!("Console Emulator ({}x{})", width, height),
        width,
        height,
        WindowOptions {
            scale,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| panic!("{}", e));
    window.set_input_callback(Box::new(CharsPressed {}));

    let tile_indices = (0..(tcols * trows)).collect::<Vec<usize>>();
    while window.is_open() && vm.can_resume() {
        let timer = Instant::now();

        Processor::with_mut(|p| {
            let mut input = 0;
            // UP
            if window.is_key_down(Key::Up) {
                input |= 1;
            }
            // DOWN
            if window.is_key_down(Key::Down) {
                input |= 1 << 1;
            }
            // LEFT
            if window.is_key_down(Key::Left) {
                input |= 1 << 2;
            }
            // RIGHT
            if window.is_key_down(Key::Right) {
                input |= 1 << 3;
            }
            // START/A
            if window.is_key_down(Key::Enter) {
                input |= 1 << 4;
            }
            // BACK/B
            if window.is_key_down(Key::Escape) {
                input |= 1 << 5;
            }
            // SELECT/X
            if window.is_key_down(Key::Space) {
                input |= 1 << 6;
            }
            // POWER/Y
            if window.is_key_down(Key::LeftShift) {
                input |= 1 << 7;
            }
            p.halt = false;
            p.input = input;
        });

        while !Processor::with(|p| p.halt)
            && vm.can_resume()
            && timer.elapsed() < Duration::from_millis(30)
        {
            vm.resume::<Processor>()
                .unwrap_or_else(|e| panic!(e.message));
        }

        Processor::with_mut(|p| p.chars_pressed.clear());

        Processor::with(|p| {
            if p.clear_screen {
                for c in &mut buffer {
                    *c = p.bg_color;
                }
            }

            if p.modes & 1 != 0 {
                // characters rendering is optimized by paralleling them because no character
                // can overlap with any other.
                tile_indices.iter().for_each(|i| {
                    let col = *i % tcols;
                    let row = *i / tcols;
                    let index = p.text_buffer[row * tcols + col];
                    draw_character(
                        &p.font[p.chars[index as usize] as usize],
                        unsafe { &mut *(buffer.as_slice() as *const [u32] as *mut [u32]) },
                        col,
                        row,
                        tcols,
                        trows,
                        p.fnt_color,
                    );
                });
            }

            if p.modes & 2 != 0 {
                // tiles rendering is optimized by paralleling them because no tile can overlap
                // with any other.
                tile_indices.par_iter().for_each(|i| {
                    let col = *i % tcols;
                    let row = *i / tcols;
                    let x = col as isize * 16 - p.tiles_viewport.0;
                    let y = row as isize * 16 - p.tiles_viewport.1;
                    let index = p.tiles[row as usize * tcols + col as usize];
                    if index > 0 {
                        draw_sprite(
                            &p.sprites[index - 1],
                            unsafe { &mut *(buffer.as_slice() as *const [u32] as *mut [u32]) },
                            x,
                            y,
                            width as isize,
                            height as isize,
                            p.blending,
                        );
                    }
                });

                // objects rendering is optimized by grouping those consecutive that does not overlap
                // any other in same group.
                let mut current = 0;
                while current < p.objects.len() {
                    let mut indices = Vec::with_capacity(p.objects.len() - current);
                    while current < p.objects.len() {
                        let object = &p.objects[current];
                        if object.index == 0 {
                            current += 1;
                        } else if !collide(&p.objects, current, &indices) {
                            indices.push(current);
                            current += 1;
                        } else {
                            break;
                        }
                    }
                    indices.par_iter().for_each(|index| {
                        let object = &p.objects[*index];
                        draw_sprite(
                            &p.sprites[object.index - 1],
                            unsafe { &mut *(buffer.as_slice() as *const [u32] as *mut [u32]) },
                            object.x - p.objects_viewport.0,
                            object.y - p.objects_viewport.1,
                            width as isize,
                            height as isize,
                            object.blending,
                        );
                    });
                }
            }
        });

        window.update_with_buffer(&buffer).unwrap();
        if let Some(rest) = Duration::from_millis(33).checked_sub(timer.elapsed()) {
            if rest > Duration::from_millis(0) {
                sleep(rest);
            }
        }
    }
}

fn collide(objects: &[Object], current: usize, others: &[usize]) -> bool {
    if others.is_empty() {
        false
    } else {
        others
            .iter()
            .any(|i| objects[current].collide(&objects[*i]))
    }
}
