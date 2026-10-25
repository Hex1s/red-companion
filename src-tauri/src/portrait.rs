//! Portraits: any common image becomes a small square lossy WebP, stored in the sheet as a data
//! URI (`Character.portrait`), so a portrait weighs kilobytes and travels with the character.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use image::imageops::FilterType;
use image::{DynamicImage, ImageDecoder, ImageReader};
use std::io::Cursor;
use rules_core::character::PORTRAIT_PREFIX;

/// Side of the stored square, px: sharp on the sheet and on retina cards, still ~10 KB.
const SIDE: u32 = 192;
/// libwebp lossy quality (0..=100).
const QUALITY: f32 = 72.0;
/// Source files above this are refused before decoding.
pub const SOURCE_MAX_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum PortraitError {
    TooLarge,
    Unreadable,
}

/// The square the GM framed, in terms independent of the picture's size: its centre as fractions
/// of the width and height, and its side as a fraction of the shorter side (1 = the biggest square).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crop {
    pub cx: f32,
    pub cy: f32,
    pub size: f32,
}

impl Crop {
    /// The biggest centred square.
    #[cfg(test)]
    pub const CENTER: Crop = Crop { cx: 0.5, cy: 0.5, size: 1.0 };

    /// `(left, top, side)` in pixels; a frame that sticks out is pushed back inside, and
    /// non-finite numbers fall back to the centre.
    fn rect(self, w: u32, h: u32) -> (u32, u32, u32) {
        let finite = |v: f32, default: f32| if v.is_finite() { v } else { default };
        let short = w.min(h);
        let side = ((finite(self.size, 1.0).clamp(0.01, 1.0) * short as f32).round() as u32).clamp(1, short);
        let place = |c: f32, len: u32| {
            let left = finite(c, 0.5) * len as f32 - side as f32 / 2.0;
            (left.round().max(0.0) as u32).min(len - side)
        };
        (place(self.cx, w), place(self.cy, h), side)
    }
}

/// Decodes `bytes` (PNG, JPEG, GIF, BMP or WebP) upright (EXIF orientation applied, as a browser
/// shows it), cuts the framed square, scales it to `SIDE`×`SIDE` and encodes it as a WebP data URI.
pub fn encode(bytes: &[u8], crop: Crop) -> Result<String, PortraitError> {
    if bytes.len() > SOURCE_MAX_BYTES {
        return Err(PortraitError::TooLarge);
    }
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| PortraitError::Unreadable)?
        .into_decoder()
        .map_err(|_| PortraitError::Unreadable)?;
    let orientation = decoder.orientation().map_err(|_| PortraitError::Unreadable)?;
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| PortraitError::Unreadable)?;
    img.apply_orientation(orientation);
    let (left, top, side) = crop.rect(img.width(), img.height());
    let square = img.crop_imm(left, top, side, side);
    let square = if side == SIDE { square } else { square.resize_exact(SIDE, SIDE, FilterType::Lanczos3) }.to_rgba8();
    let webp = webp::Encoder::from_rgba(square.as_raw(), SIDE, SIDE).encode(QUALITY);
    Ok(format!("{PORTRAIT_PREFIX}{}", STANDARD.encode(&*webp)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba, RgbaImage};
    use rules_core::character::{Character, PORTRAIT_MAX_LEN};
    use std::io::Cursor;

    fn png(w: u32, h: u32) -> Vec<u8> {
        // A gradient: flat colour would compress to nothing and prove little.
        let img = RgbaImage::from_fn(w, h, |x, y| Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255]));
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// Left half red, right half blue.
    fn two_halves(w: u32, h: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(w, h, |x, _| if x < w / 2 { Rgba([255, 0, 0, 255]) } else { Rgba([0, 0, 255, 255]) });
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    /// Red and blue of the portrait's centre pixel.
    fn centre_rb(uri: &str) -> (u8, u8) {
        let raw = STANDARD.decode(&uri[PORTRAIT_PREFIX.len()..]).unwrap();
        let px = image::load_from_memory_with_format(&raw, ImageFormat::WebP).unwrap().to_rgba8();
        let p = px.get_pixel(SIDE / 2, SIDE / 2);
        (p[0], p[2])
    }

    #[test]
    fn the_frame_picks_which_part_of_the_picture_is_kept() {
        let src = two_halves(400, 200);
        // A square of half the short side, centred on the left half / the right half.
        let left = encode(&src, Crop { cx: 0.25, cy: 0.5, size: 0.5 }).unwrap();
        let right = encode(&src, Crop { cx: 0.75, cy: 0.5, size: 0.5 }).unwrap();
        let (r, b) = centre_rb(&left);
        assert!(r > 200 && b < 60, "left frame should be red: {r} {b}");
        let (r, b) = centre_rb(&right);
        assert!(b > 200 && r < 60, "right frame should be blue: {r} {b}");
    }

    #[test]
    fn a_frame_sticking_out_is_pushed_back_inside() {
        // 400x200: the biggest square is 200 wide; a centre at 0 would start at -100.
        assert_eq!(Crop { cx: 0.0, cy: 0.5, size: 1.0 }.rect(400, 200), (0, 0, 200));
        assert_eq!(Crop { cx: 1.0, cy: 9.0, size: 1.0 }.rect(400, 200), (200, 0, 200));
        // Silly numbers never panic and never leave the picture.
        assert_eq!(Crop { cx: f32::NAN, cy: f32::INFINITY, size: -3.0 }.rect(400, 200), (199, 99, 2));
        assert_eq!(Crop::CENTER.rect(300, 300), (0, 0, 300));
    }

    #[test]
    fn a_big_picture_becomes_a_small_valid_square_webp() {
        let uri = encode(&png(1600, 900), Crop::CENTER).unwrap();
        assert!(uri.len() < PORTRAIT_MAX_LEN, "{} chars", uri.len());
        let raw = STANDARD.decode(&uri[PORTRAIT_PREFIX.len()..]).unwrap();
        let back = image::load_from_memory_with_format(&raw, ImageFormat::WebP).unwrap();
        assert_eq!((back.width(), back.height()), (SIDE, SIDE));

        // The sheet accepts what the encoder produces.
        let mut c = Character::quick("V");
        c.portrait = Some(uri);
        assert_eq!(c.validate(), Ok(()));
    }

    #[test]
    fn garbage_and_huge_files_are_refused() {
        assert_eq!(encode(b"not an image", Crop::CENTER), Err(PortraitError::Unreadable));
        assert_eq!(encode(&vec![0; SOURCE_MAX_BYTES + 1], Crop::CENTER), Err(PortraitError::TooLarge));
    }
}
