use byteorder::{BigEndian, ReadBytesExt};
use core::error::*;
use std::io::{Cursor, Read};
use vm_core::vm::Vm;

pub type Sprite = Vec<u8>;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Into<u32> for Color {
    fn into(self) -> u32 {
        u32::from(self.b) | (u32::from(self.g) << 8) | (u32::from(self.r) << 16)
    }
}

type ReadCartridge = (Vec<Sprite>, Vec<u8>, Vm, (usize, usize));

pub fn read_cartridge(bytes: &[u8]) -> SimpleResult<ReadCartridge> {
    let mut stream = Cursor::new(bytes);
    let mut header = vec![0; 4];
    stream.read_exact(&mut header)?;
    if header != [67u8, 65u8, 82u8, 84u8] {
        return Err(SimpleError::new(
            "Cartridge does not have proper header".to_owned(),
        ));
    }
    let mode = stream.read_u8()?;
    let wh = match mode {
        0 => (320, 240),
        1 => (160, 128),
        _ => {
            return Err(SimpleError::new(format!(
                "Unsupported graphics mode: {}",
                mode
            )))
        }
    };
    let count = stream.read_u8()? as usize;
    let mut colors = Vec::with_capacity(count);
    for _ in 0..count {
        colors.push(Color {
            r: stream.read_u8()?,
            g: stream.read_u8()?,
            b: stream.read_u8()?,
        });
    }
    let count = stream.read_u8()? as usize;
    let mut sprites = Vec::with_capacity(count);
    for _ in 0..count {
        let mut sprite = vec![0; 16 * 16];
        stream.read_exact(&mut sprite)?;
        sprites.push(decompress_sprite(&sprite, &colors));
    }
    let size = stream.read_u16::<BigEndian>()? as usize;
    let mut data = vec![0; size];
    stream.read_exact(&mut data)?;
    let size = stream.read_u64::<BigEndian>()? as usize;
    let mut buffer = vec![0; size];
    stream.read_exact(&mut buffer)?;
    let vm = Vm::from_bytes(buffer, 1024 * 4, 1024 * 60)?;
    Ok((sprites, data, vm, wh))
}

fn decompress_sprite(bytes: &[u8], colors: &[Color]) -> Sprite {
    unsafe {
        let result = vec![0u8; bytes.len() * 3];
        let target = result.as_ptr() as *mut Color;
        for i in 0..bytes.len() {
            *target.add(i) = colors[bytes[i] as usize];
        }
        result
    }
}
