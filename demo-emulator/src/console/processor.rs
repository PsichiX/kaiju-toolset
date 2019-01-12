use crate::cartridge::Color;
use crate::render::Blending;
use core::error::*;
use rand::prelude::*;
use std::fs::{read, write};
use std::path::Path;
use std::sync::Mutex;
use vm_core::load_cstring;
use vm_core::processor::{OpAction, Processor as VmProcessor};
use vm_core::state::Value;
use vm_core::vm::Vm;

lazy_static! {
    static ref PROC: Mutex<Processor> = Mutex::new(Processor::new());
}

#[derive(Debug, Copy, Clone)]
pub struct Object {
    pub index: usize,
    pub x: isize,
    pub y: isize,
    pub blending: Blending,
}

impl Object {
    pub fn new(index: usize, x: isize, y: isize, blending: Blending) -> Self {
        Self {
            index,
            x,
            y,
            blending,
        }
    }

    pub fn collide(&self, other: &Self) -> bool {
        self.x + 16 > other.x
            && self.x < other.x + 16
            && self.y + 16 > other.y
            && self.y < other.y + 16
    }
}

fn overlap(a: (i16, i16, i16, i16), b: (i16, i16, i16, i16)) -> bool {
    a.0 + a.2 > b.0 && a.0 < b.0 + b.2 && a.1 + a.3 > b.1 && a.1 < b.1 + b.3
}

pub struct Processor {
    pub modes: u8,
    pub tcols: usize,
    pub trows: usize,
    pub sprites: Vec<Vec<u8>>,
    pub tiles: Vec<usize>,
    pub tiles_viewport: (isize, isize),
    pub objects: Vec<Object>,
    pub objects_viewport: (isize, isize),
    pub blending: Blending,
    pub clear_screen: bool,
    pub halt: bool,
    pub input: u8,
    pub data_block: Value,
    pub save_file: String,
    pub bg_color: u32,
    pub fnt_color: u32,
    pub font: Vec<Vec<u8>>,
    pub chars: Vec<u8>,
    pub text_buffer: Vec<u8>,
    pub text_cols: usize,
    pub text_rows: usize,
    pub text_col: usize,
    pub text_row: usize,
    pub chars_pressed: Vec<u8>,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            modes: 1,
            tcols: 0,
            trows: 0,
            sprites: vec![],
            tiles: vec![],
            tiles_viewport: (0, 0),
            objects: ::std::iter::repeat(Object::new(0, 0, 0, Blending::None))
                .take(600)
                .collect::<Vec<Object>>(),
            objects_viewport: (0, 0),
            blending: Blending::None,
            clear_screen: true,
            halt: false,
            input: 0,
            data_block: Value::default(),
            save_file: "./save.dat".to_owned(),
            bg_color: 0,
            fnt_color: 0xFFFF_FFFF,
            font: vec![],
            chars: vec![],
            text_buffer: vec![],
            text_cols: 0,
            text_rows: 0,
            text_col: 0,
            text_row: 0,
            chars_pressed: Vec::with_capacity(1024),
        }
    }

    pub fn with<T, F>(mut cb: F) -> T
    where
        F: FnMut(&Self) -> T,
    {
        cb(&PROC.lock().unwrap())
    }

    pub fn with_mut<T, F>(mut cb: F) -> T
    where
        F: FnMut(&mut Self) -> T,
    {
        cb(&mut PROC.lock().unwrap())
    }
}

impl VmProcessor for Processor {
    #[allow(clippy::ptr_arg)]
    fn process_op(
        op: &String,
        params: &Vec<usize>,
        targets: &Vec<usize>,
        vm: &mut Vm,
    ) -> SimpleResult<OpAction> {
        match op.as_str() {
            "halt" => {
                Self::with_mut(|p| p.halt = true);
                Ok(OpAction::None)
            }
            "goto" => {
                let v = load_cstring(params[0], vm)?;
                if let Some(pos) = vm.find_label(&v) {
                    Ok(OpAction::GoTo(pos))
                } else {
                    Err(SimpleError::new(format!(
                        "Function does not have `{}` label",
                        v
                    )))
                }
            }
            "if" => {
                let v = vm.state().load_data::<i16>(params[0])?;
                let th = load_cstring(params[1], vm)?;
                let th = if let Some(pos) = vm.find_label(&th) {
                    pos
                } else {
                    return Err(SimpleError::new(format!(
                        "Function does not have `{}` label",
                        v
                    )));
                };
                let el = load_cstring(params[2], vm)?;
                let el = if let Some(pos) = vm.find_label(&el) {
                    pos
                } else {
                    return Err(SimpleError::new(format!(
                        "Function does not have `{}` label",
                        v
                    )));
                };
                Ok(OpAction::GoTo(if v != 0 { th } else { el }))
            }
            "ret" => Ok(OpAction::Return),
            "pass" => Ok(OpAction::None),
            "dbgi" => {
                let v = vm.state().load_data::<i16>(params[0])?;
                println!("{}", v);
                Ok(OpAction::None)
            }
            "dbgs" => {
                let v = load_cstring(params[0], vm)?;
                println!("{}", v);
                Ok(OpAction::None)
            }
            "dbgp" => {
                let p = vm.state().load_data::<usize>(params[0])?;
                println!("{:#X} ({})", p, p);
                Ok(OpAction::None)
            }
            "dbgm" => {
                let a = vm.state().load_data::<usize>(params[0])?;
                let s = vm.state().load_data::<i16>(params[1])? as usize;
                println!("{:?}", vm.state().load_bytes(a, s)?);
                Ok(OpAction::None)
            }
            "test" => {
                println!("<stack: {}>", vm.state().stack_pos());
                Ok(OpAction::None)
            }
            "cstp" => {
                let a = vm.state().load_data::<usize>(params[0])?;
                vm.state_mut().store_data(targets[0], &a)?;
                Ok(OpAction::None)
            }
            "poff" => {
                let a = vm.state().load_data::<usize>(params[0])?;
                let o = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut()
                    .store_data(targets[0], &((a as isize + o as isize) as usize))?;
                Ok(OpAction::None)
            }
            "i2b" => {
                let v = vm.state().load_data::<i16>(params[0])?;
                vm.state_mut().store_data(targets[0], &(v as u8))?;
                Ok(OpAction::None)
            }
            "b2i" => {
                let v = vm.state().load_data::<u8>(params[0])?;
                vm.state_mut().store_data(targets[0], &i16::from(v))?;
                Ok(OpAction::None)
            }
            "allc" => {
                let size = vm.state().load_data::<i16>(params[0])? as usize;
                let v = vm.state_mut().alloc_memory_value(size + 2)?;
                vm.state_mut().store_data(v.address, &(v.size as u16))?;
                vm.state_mut().store_data(targets[0], &(v.address + 2))?;
                Ok(OpAction::None)
            }
            "free" => {
                let address = vm.state().load_data::<usize>(params[0])?;
                let size = vm.state().load_data::<u16>(address - 2)?;
                vm.state_mut()
                    .dealloc_memory_value(&Value::new(address - 2, size as usize))?;
                Ok(OpAction::None)
            }
            "mode" => {
                let f = vm.state().load_data::<usize>(params[0])?;
                Self::with_mut(|p| p.modes = f as u8);
                Ok(OpAction::None)
            }
            "inp" => {
                let input = Self::with_mut(|p| p.input);
                vm.state_mut().store_data(targets[0], &input)?;
                Ok(OpAction::None)
            }
            "data" => {
                let address = Self::with_mut(|p| p.data_block.address);
                vm.state_mut().store_data(targets[0], &address)?;
                Ok(OpAction::None)
            }
            "tls" => {
                let tile = vm.state().load_data::<i16>(params[0])? as usize;
                if tile >= Self::with(|p| p.tiles.len()) {
                    return Err(SimpleError::new(format!(
                        "Trying to access tile out of bounds: {}",
                        tile,
                    )));
                }
                let col = vm.state().load_data::<i16>(params[1])? as usize;
                let row = vm.state().load_data::<i16>(params[2])? as usize;
                Self::with_mut(|p| p.tiles[row * p.tcols + col] = tile as usize);
                Ok(OpAction::None)
            }
            "tln" => {
                Self::with_mut(|p| p.blending = Blending::None);
                Ok(OpAction::None)
            }
            "tlb" => {
                let xr = vm.state().load_data::<i16>(params[0])?;
                let gb = vm.state().load_data::<i16>(params[1])?;
                let r = (xr & 0xFF) as u8;
                let g = (gb >> 8) as u8;
                let b = (gb & 0xFF) as u8;
                Self::with_mut(|p| p.blending = Blending::Key(Color { r, g, b }));
                Ok(OpAction::None)
            }
            "tlv" => {
                let x = vm.state().load_data::<i16>(params[0])? as isize;
                let y = vm.state().load_data::<i16>(params[1])? as isize;
                Self::with_mut(|p| p.tiles_viewport = (x, y));
                Ok(OpAction::None)
            }
            "objs" => {
                let index = vm.state().load_data::<i16>(params[0])? as usize;
                if index >= Self::with(|p| p.objects.len()) {
                    return Err(SimpleError::new(format!(
                        "Trying to access object out of bounds: {}",
                        index,
                    )));
                }
                let sprite = vm.state().load_data::<i16>(params[1])? as usize;
                Self::with_mut(|p| p.objects[index].index = sprite as usize);
                Ok(OpAction::None)
            }
            "objp" => {
                let index = vm.state().load_data::<i16>(params[0])? as usize;
                if index >= Self::with(|p| p.objects.len()) {
                    return Err(SimpleError::new(format!(
                        "Trying to access object out of bounds: {}",
                        index,
                    )));
                }
                let x = vm.state().load_data::<i16>(params[1])?;
                let y = vm.state().load_data::<i16>(params[2])?;
                Self::with_mut(|p| {
                    p.objects[index].x = x as isize;
                    p.objects[index].y = y as isize;
                });
                Ok(OpAction::None)
            }
            "objn" => {
                let index = vm.state().load_data::<i16>(params[0])? as usize;
                if index >= Self::with(|p| p.objects.len()) {
                    return Err(SimpleError::new(format!(
                        "Trying to access object out of bounds: {}",
                        index,
                    )));
                }
                Self::with_mut(|p| p.objects[index].blending = Blending::None);
                Ok(OpAction::None)
            }
            "objb" => {
                let index = vm.state().load_data::<i16>(params[0])? as usize;
                if index >= Self::with(|p| p.objects.len()) {
                    return Err(SimpleError::new(format!(
                        "Trying to access object out of bounds: {}",
                        index,
                    )));
                }
                let xr = vm.state().load_data::<i16>(params[1])?;
                let gb = vm.state().load_data::<i16>(params[2])?;
                let r = (xr & 0xFF) as u8;
                let g = (gb >> 8) as u8;
                let b = (gb & 0xFF) as u8;
                Self::with_mut(|p| p.objects[index].blending = Blending::Key(Color { r, g, b }));
                Ok(OpAction::None)
            }
            "objv" => {
                let x = vm.state().load_data::<i16>(params[0])? as isize;
                let y = vm.state().load_data::<i16>(params[1])? as isize;
                Self::with_mut(|p| p.objects_viewport = (x, y));
                Ok(OpAction::None)
            }
            "std" => {
                let address = vm.state().load_data::<usize>(params[0])?;
                let size = vm.state().load_data::<i16>(params[1])? as usize;
                let bytes = vm.state().load_bytes(address, size)?;
                Self::with_mut(|p| write(&p.save_file, &bytes))?;
                Ok(OpAction::None)
            }
            "ldd" => {
                let address = vm.state().load_data::<usize>(params[0])?;
                let size = vm.state().load_data::<i16>(params[1])? as usize;
                let bytes = Self::with_mut(|p| read(&p.save_file))?;
                vm.state_mut().store_bytes(address, &bytes[0..size])?;
                Ok(OpAction::None)
            }
            "hd" => {
                let exists = Self::with_mut(|p| Path::new(&p.save_file).is_file());
                vm.state_mut()
                    .store_data(targets[0], if exists { &1i16 } else { &0i16 })?;
                Ok(OpAction::None)
            }
            "bgc" => {
                let xr = vm.state().load_data::<i16>(params[0])?;
                let gb = vm.state().load_data::<i16>(params[1])?;
                let color = gb as u32 | ((xr as u32) << 16);
                Self::with_mut(|p| p.bg_color = color);
                Ok(OpAction::None)
            }
            "fgc" => {
                let xr = vm.state().load_data::<i16>(params[0])? as u32;
                let gb = vm.state().load_data::<i16>(params[1])? as u32;
                let color = gb as u32 | ((xr as u32) << 16);
                Self::with_mut(|p| p.fnt_color = color);
                Ok(OpAction::None)
            }
            "chr" => {
                let c = vm.state().load_data::<u8>(params[0])?;
                let x = vm.state().load_data::<i16>(params[1])? as usize;
                let y = vm.state().load_data::<i16>(params[2])? as usize;
                Self::with_mut(|p| p.text_buffer[y * p.text_cols + x] = c);
                Ok(OpAction::None)
            }
            "kcc" => {
                let c = Self::with(|p| p.chars_pressed.len() as i16);
                vm.state_mut().store_data(targets[0], &c)?;
                Ok(OpAction::None)
            }
            "gkc" => {
                let index = vm.state().load_data::<i16>(params[0])?;
                let c = Self::with(|p| p.chars_pressed[index as usize]);
                vm.state_mut().store_data(targets[0], &c)?;
                Ok(OpAction::None)
            }
            "ovlp" => {
                let a = vm.state().load_data::<(i16, i16, i16, i16)>(params[0])?;
                let b = vm.state().load_data::<(i16, i16, i16, i16)>(params[1])?;
                vm.state_mut()
                    .store_data(targets[0], &if overlap(a, b) { 1i16 } else { 0i16 })?;
                Ok(OpAction::None)
            }
            "add" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a + b))?;
                Ok(OpAction::None)
            }
            "sub" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a - b))?;
                Ok(OpAction::None)
            }
            "mul" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a * b))?;
                Ok(OpAction::None)
            }
            "div" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a / b))?;
                Ok(OpAction::None)
            }
            "mod" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a % b))?;
                Ok(OpAction::None)
            }
            "mov" => {
                let v = vm.state().load_data::<i16>(params[0])?;
                vm.state_mut().store_data(targets[0], &v)?;
                Ok(OpAction::None)
            }
            "eq" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut()
                    .store_data(targets[0], if a == b { &1i16 } else { &0i16 })?;
                Ok(OpAction::None)
            }
            "nq" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut()
                    .store_data(targets[0], if a != b { &1i16 } else { &0i16 })?;
                Ok(OpAction::None)
            }
            "gt" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut()
                    .store_data(targets[0], if a > b { &1i16 } else { &0i16 })?;
                Ok(OpAction::None)
            }
            "lt" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut()
                    .store_data(targets[0], if a < b { &1i16 } else { &0i16 })?;
                Ok(OpAction::None)
            }
            "ge" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut()
                    .store_data(targets[0], if a >= b { &1i16 } else { &0i16 })?;
                Ok(OpAction::None)
            }
            "le" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut()
                    .store_data(targets[0], if a <= b { &1i16 } else { &0i16 })?;
                Ok(OpAction::None)
            }
            "lsh" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a << b))?;
                Ok(OpAction::None)
            }
            "rsh" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a >> b))?;
                Ok(OpAction::None)
            }
            "and" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a & b))?;
                Ok(OpAction::None)
            }
            "or" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a | b))?;
                Ok(OpAction::None)
            }
            "xor" => {
                let a = vm.state().load_data::<i16>(params[0])?;
                let b = vm.state().load_data::<i16>(params[1])?;
                vm.state_mut().store_data(targets[0], &(a ^ b))?;
                Ok(OpAction::None)
            }
            "neg" => {
                let v = vm.state().load_data::<i16>(params[0])?;
                vm.state_mut().store_data(targets[0], &(!v))?;
                Ok(OpAction::None)
            }
            "rnd" => {
                vm.state_mut().store_data::<i16>(targets[0], &random())?;
                Ok(OpAction::None)
            }
            _ => Err(SimpleError::new(format!("Unsupported op: {}", op))),
        }
    }
}
