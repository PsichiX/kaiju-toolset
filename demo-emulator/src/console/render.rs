use crate::cartridge::Color;

#[derive(Debug, Copy, Clone)]
pub enum Blending {
    None,
    Key(Color),
}

pub fn draw_character(
    character: &[u8],
    target: &mut [u32],
    col: usize,
    row: usize,
    cols: usize,
    rows: usize,
    color: u32,
) {
    if col < cols && row < rows {
        let w = cols * 8;
        for sy in 0..8 {
            let y = row * 8 + sy;
            for sx in 0..8 {
                let x = col * 8 + sx;
                let pos = (y * w + x) as usize;
                if character[sy * 8 + sx] > 0 {
                    target[pos] = color;
                }
            }
        }
    }
}

pub fn draw_sprite(
    sprite: &[u8],
    target: &mut [u32],
    tx: isize,
    ty: isize,
    tw: isize,
    th: isize,
    blend: Blending,
) {
    if tx + 16 > 0 && ty + 16 > 0 && tx < tw && ty < th {
        unsafe {
            let sprite = sprite.as_ptr() as *const Color;
            for sy in 0..16 {
                let y = ty + sy;
                if y >= 0 && y < th {
                    for sx in 0..16 {
                        let x = tx + sx;
                        if x >= 0 && x < tw {
                            let pos = (y * tw + x) as usize;
                            let source = *sprite.add((sy * 16 + sx) as usize);
                            match blend {
                                Blending::None => {
                                    target[pos] = source.into();
                                }
                                Blending::Key(c) => {
                                    if c != source {
                                        target[pos] = source.into();
                                    }
                                }
                            };
                        }
                    }
                }
            }
        }
    }
}
