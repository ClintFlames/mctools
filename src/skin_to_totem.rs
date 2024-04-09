use std::{
    fs::File,
    io::{BufWriter, Read},
    path::Path,
};

struct Point(u8, u8);

#[derive(Debug)]
struct PngVec(Vec<Vec<u32>>);

impl PngVec {
    fn new(size: Point) -> Self {
        PngVec(vec![vec![0; size.1 as usize]; size.0 as usize])
    }

    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let decoder = png::Decoder::new(match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                return Err(e.to_string());
            }
        });

        let mut reader = match decoder.read_info() {
            Ok(r) => r,
            Err(e) => {
                return Err(e.to_string());
            }
        };

        let mut buf = vec![0; reader.output_buffer_size()];

        let info = match reader.next_frame(&mut buf) {
            Ok(i) => i,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        let bytes = &buf[..info.buffer_size()];
        let mut pngvec = PngVec(vec![vec![0; info.height as usize]; info.width as usize]);

        if info.bit_depth != png::BitDepth::Eight {
            return Err(String::from("BitDepth must be 8"));
        }

        let s = bytes.len() as usize / (info.width * info.height) as usize;
        let palette = if png::ColorType::Indexed == info.color_type {
            match &reader.info().palette {
                Some(p) => p.to_vec(),
                None => return Err(String::from("Not found pallete for Indexed png")),
            }
        } else {
            vec![]
        };

        for x in 0..info.width as usize {
            for y in 0..info.height as usize {
                let i = ((y * info.width as usize) + x) * s;
                pngvec.0[x][y] = match info.color_type {
                    png::ColorType::Rgb => {
                        let r = bytes[i];
                        let g = bytes[i + 1];
                        let b = bytes[i + 2];

                        if r == 0 && g == 0 && b == 0 {
                            0
                        } else {
                            u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], 0xff])
                        }
                    }
                    png::ColorType::Rgba => {
                        u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]])
                    }
                    png::ColorType::Indexed => {
                        let pi = (bytes[i] * 3) as usize;
                        let r = palette[pi];
                        let g = palette[pi + 1];
                        let b = palette[pi + 2];

                        if r <= 1 && g <= 1 && b <= 1 {
                            0
                        } else {
                            u32::from_be_bytes([r, g, b, 0xff])
                        }
                    }
                    png::ColorType::Grayscale => {
                        if bytes[i] <= 1 {
                            0
                        } else {
                            u32::from_be_bytes([bytes[i], bytes[i], bytes[i], 0xff])
                        }
                    }
                    png::ColorType::GrayscaleAlpha => {
                        u32::from_be_bytes([bytes[i], bytes[i], bytes[i], bytes[i + 1]])
                    }
                };
            }
        }

        Ok(pngvec)
    }

    fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let file = match File::create(path) {
            Ok(f) => f,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        let ref mut buf = BufWriter::new(file);

        let mut encoder = png::Encoder::new(buf, self.0.len() as u32, self.0[0].len() as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = match encoder.write_header() {
            Ok(w) => w,
            Err(e) => {
                return Err(e.to_string());
            }
        };

        let mut pngdata: Vec<u8> = Vec::new();

        for y in 0..self.0[0].len() {
            for x in 0..self.0.len() {
                let col = u32::to_be_bytes(self.0[x][y]);
                pngdata.push(col[0]);
                pngdata.push(col[1]);
                pngdata.push(col[2]);
                pngdata.push(col[3]);
            }
        }

        match writer.write_image_data(&pngdata) {
            Ok(_) => (),
            Err(e) => {
                return Err(e.to_string());
            }
        };

        Ok(())
    }

    fn add_colors(hex1: u32, hex2: u32) -> u32 {
        if hex1 == 0 || hex2 & 0xff == 0xff {
            return hex2;
        } else if hex2 == 0 {
            return hex1;
        }
        let rgba1 = hex1.to_be_bytes();
        let rgba2 = hex2.to_be_bytes();

        let alpha1 = rgba1[3] as f32;
        let alpha2 = rgba2[3] as f32;

        let alpha_final = alpha1 + alpha2 * (1.0 - alpha1);
        let r =
            (rgba1[0] as f32 * alpha1 + rgba2[0] as f32 * alpha2 * (1.0 - alpha1)) / alpha_final;
        let g =
            (rgba1[1] as f32 * alpha1 + rgba2[1] as f32 * alpha2 * (1.0 - alpha1)) / alpha_final;
        let b =
            (rgba1[2] as f32 * alpha1 + rgba2[2] as f32 * alpha2 * (1.0 - alpha1)) / alpha_final;

        let bytes = [r as u8, g as u8, b as u8, alpha_final as u8];

        u32::from_be_bytes(bytes)
    }

    fn clear_rect(&mut self, pos: Point, size: Point) -> Result<(), String> {
        if size.0 == 0 || size.1 == 0 {
            return Err(String::from("Size can't be 0"));
        }
        match pos.0.checked_add(size.0) {
            Some(_) => (),
            None => {
                return Err(String::from("pos.0 + size.0 overflows"));
            }
        }
        match pos.1.checked_add(size.1) {
            Some(_) => (),
            None => {
                return Err(String::from("pos.1 + size.1 overflows"));
            }
        }

        for x in pos.0..pos.0 + size.0 {
            for y in pos.1..pos.1 + size.1 {
                self.0[x as usize][y as usize] = 0;
            }
        }

        Ok(())
    }
    fn draw_image(
        &mut self,
        target: &PngVec,
        tpos: Point,
        tsize: Point,
        pos: Point,
    ) -> Result<(), String> {
        if tsize.0 == 0 || tsize.1 == 0 {
            return Err(String::from("tsize can't be 0"));
        }
        match tpos.0.checked_add(tsize.0) {
            Some(_) => (),
            None => {
                return Err(String::from("tpos.0 + tsize.0 overflows"));
            }
        }
        match tpos.1.checked_add(tsize.1) {
            Some(_) => (),
            None => {
                return Err(String::from("tpos.1 + tsize.1 overflows"));
            }
        }
        match pos.0.checked_add(tsize.0) {
            Some(_) => (),
            None => {
                return Err(String::from("pos.0 + tsize.0 overflows"));
            }
        }
        match pos.1.checked_add(tsize.1) {
            Some(_) => (),
            None => {
                return Err(String::from("pos.1 + tsize.1 overflows"));
            }
        }

        for x in 0..tsize.0 {
            for y in 0..tsize.1 {
                let tx = (tpos.0 + x) as usize;
                let ty = (tpos.1 + y) as usize;
                let cx = (pos.0 + x) as usize;
                let cy = (pos.1 + y) as usize;

                self.0[cx][cy] = PngVec::add_colors(self.0[cx][cy], target.0[tx][ty]);
            }
        }

        Ok(())
    }
}

pub fn generate<P: AsRef<Path>>(
    skin_path: P,
    totem_path: P,
    second_layer: bool,
) -> Result<(), String> {
    let skin = match PngVec::from_file(skin_path) {
        Ok(p) => {
            if p.0.len() != 64 || p.0[0].len() != 64 {
                return Err(String::from("Skin must be 64x64"));
            }
            p
        }
        Err(e) => {
            return Err(String::from(e.to_string()));
        }
    };
    let mut totem = PngVec::new(Point(16, 16));

    totem.draw_image(&skin, Point(8, 8), Point(8, 8), Point(4, 1))?;
    totem.clear_rect(Point(4, 1), Point(1, 1))?;
    totem.clear_rect(Point(11, 1), Point(1, 1))?;
    totem.draw_image(&skin, Point(20, 21), Point(8, 1), Point(4, 9))?;
    totem.draw_image(&skin, Point(20, 23), Point(8, 1), Point(4, 10))?;
    totem.draw_image(&skin, Point(20, 29), Point(8, 1), Point(4, 11))?;
    totem.draw_image(&skin, Point(20, 31), Point(8, 1), Point(4, 12))?;
    totem.draw_image(&skin, Point(5, 20), Point(3, 2), Point(5, 13))?;
    totem.draw_image(&skin, Point(6, 31), Point(2, 1), Point(6, 15))?;

    totem.draw_image(&skin, Point(20, 52), Point(3, 2), Point(8, 13))?;
    totem.draw_image(&skin, Point(20, 63), Point(2, 1), Point(8, 15))?;

    totem.draw_image(&skin, Point(44, 20), Point(1, 1), Point(3, 8))?;
    totem.draw_image(&skin, Point(45, 20), Point(1, 1), Point(3, 9))?;
    totem.draw_image(&skin, Point(46, 20), Point(1, 1), Point(3, 10))?;
    totem.draw_image(&skin, Point(44, 21), Point(1, 1), Point(2, 8))?;
    totem.draw_image(&skin, Point(45, 21), Point(1, 1), Point(2, 9))?;
    totem.draw_image(&skin, Point(46, 21), Point(1, 1), Point(2, 10))?;
    totem.draw_image(&skin, Point(44, 31), Point(1, 1), Point(1, 8))?;
    totem.draw_image(&skin, Point(45, 31), Point(1, 1), Point(1, 9))?;

    totem.draw_image(&skin, Point(39, 52), Point(1, 1), Point(12, 8))?;
    totem.draw_image(&skin, Point(38, 52), Point(1, 1), Point(12, 9))?;
    totem.draw_image(&skin, Point(37, 52), Point(1, 1), Point(12, 10))?;
    totem.draw_image(&skin, Point(39, 53), Point(1, 1), Point(13, 8))?;
    totem.draw_image(&skin, Point(38, 53), Point(1, 1), Point(13, 9))?;
    totem.draw_image(&skin, Point(37, 53), Point(1, 1), Point(13, 10))?;
    totem.draw_image(&skin, Point(37, 63), Point(1, 1), Point(14, 8))?;
    totem.draw_image(&skin, Point(38, 63), Point(1, 1), Point(14, 9))?;

    if second_layer {
        totem.draw_image(&skin, Point(40, 8), Point(8, 8), Point(4, 1))?;
        totem.draw_image(&skin, Point(44, 36), Point(1, 1), Point(3, 8))?;
        totem.draw_image(&skin, Point(45, 36), Point(1, 1), Point(3, 9))?;
        totem.draw_image(&skin, Point(46, 36), Point(1, 1), Point(3, 10))?;
        totem.draw_image(&skin, Point(44, 37), Point(1, 1), Point(2, 8))?;
        totem.draw_image(&skin, Point(45, 37), Point(1, 1), Point(2, 9))?;
        totem.draw_image(&skin, Point(46, 37), Point(1, 1), Point(2, 10))?;
        totem.draw_image(&skin, Point(44, 47), Point(1, 1), Point(1, 8))?;
        totem.draw_image(&skin, Point(45, 47), Point(1, 1), Point(1, 9))?;
        totem.draw_image(&skin, Point(55, 52), Point(1, 1), Point(12, 8))?;
        totem.draw_image(&skin, Point(54, 52), Point(1, 1), Point(12, 9))?;
        totem.draw_image(&skin, Point(53, 52), Point(1, 1), Point(12, 10))?;
        totem.draw_image(&skin, Point(55, 53), Point(1, 1), Point(13, 8))?;
        totem.draw_image(&skin, Point(54, 53), Point(1, 1), Point(13, 9))?;
        totem.draw_image(&skin, Point(53, 53), Point(1, 1), Point(13, 10))?;
        totem.draw_image(&skin, Point(53, 63), Point(1, 1), Point(14, 8))?;
        totem.draw_image(&skin, Point(54, 63), Point(1, 1), Point(14, 9))?;
        totem.draw_image(&skin, Point(20, 37), Point(8, 1), Point(4, 9))?;
        totem.draw_image(&skin, Point(20, 39), Point(8, 1), Point(4, 10))?;
        totem.draw_image(&skin, Point(20, 45), Point(8, 1), Point(4, 11))?;
        totem.draw_image(&skin, Point(20, 47), Point(8, 1), Point(4, 12))?;
        totem.draw_image(&skin, Point(5, 36), Point(3, 2), Point(5, 13))?;
        totem.draw_image(&skin, Point(6, 47), Point(2, 1), Point(6, 15))?;
        totem.draw_image(&skin, Point(4, 52), Point(3, 2), Point(8, 13))?;
        totem.draw_image(&skin, Point(4, 63), Point(2, 1), Point(8, 15))?;
    }

    totem.save(totem_path)?;
    Ok(())
}

// TODO from ETF skin/or additional data
// pub fn generate_animated<P: AsRef<Path>>(
//     skin_path: P,
//     totem_path: P,
//     second_layer: bool,
// ) -> Result<(), String> {
//     Ok(())
// }
