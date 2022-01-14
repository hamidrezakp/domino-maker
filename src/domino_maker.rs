use image::{
    imageops::FilterType, io::Reader as ImageReader, jpeg::JpegEncoder, ColorType, GenericImage,
    ImageBuffer, ImageEncoder, Luma, Pixel,
};
use std::io::Cursor;

const DOMINO_WIDTH: u32 = 8;
const DOMINO_HEIGHT: u32 = 24;

fn get_final_image_size(board_size: (u32, u32)) -> (u32, u32) {
    let width = DOMINO_WIDTH * (2 * board_size.0 - 1);
    let height = DOMINO_WIDTH * (4 * board_size.1 - 1);
    (width, height)
}

fn make_pixel_color(between: bool, sum_of_area: u32) -> u8 {
    match between {
        true => 255,
        false if (sum_of_area / DOMINO_WIDTH * DOMINO_HEIGHT) < 128 => 0,
        _ => 255,
    }
}

fn simplify_image(img: &mut ImageBuffer<Luma<u8>, Vec<u8>>) {
    img.inner_mut()
        .pixels_mut()
        .for_each(|i| i.apply(|j| if j < 128 { 0 } else { 255 }));
}

pub struct ConvertResult {
    pub bytes: Vec<u8>,
    pub white_count: u32,
    pub black_count: u32,
}

pub fn convert(bytes: &[u8], board_size: (u32, u32)) -> Result<ConvertResult, &'static str> {
    let img = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "invalid bytes")?
        .decode()
        .map_err(|_| "unknown image type")?;

    let (width, height) = get_final_image_size(board_size);

    let mut img = img
        .resize_exact(width, height, FilterType::Nearest)
        .to_luma8();

    simplify_image(&mut img);

    let mut i = 0;
    let mut j = 0;
    let mut between = false;

    let mut white_count = 0;
    let mut black_count = 0;

    while j < height {
        let mut sum = 0;

        for x in 0..DOMINO_WIDTH {
            for y in 0..DOMINO_HEIGHT {
                sum = sum + (img.get_pixel(i + x, j + y).channels()[0]) as u32;
            }
        }

        let pixel = match make_pixel_color(between, sum) {
            0 if !between => {
                black_count = black_count + 1;
                0
            }
            255 if !between => {
                white_count = white_count + 1;
                255
            }
            0 => 0,
            255 => 255,
            _ => panic!("invalid color"),
        };

        for x in 0..DOMINO_WIDTH {
            for y in 0..DOMINO_HEIGHT {
                img.get_pixel_mut(i + x, j + y).apply(|_| pixel);
            }
        }

        if j != (height - DOMINO_HEIGHT) {
            for x in 0..DOMINO_WIDTH {
                for y in 0..DOMINO_WIDTH {
                    img.get_pixel_mut(i + x, j + DOMINO_HEIGHT + y)
                        .apply(|_| 255);
                }
            }
        }

        if i == (width - DOMINO_WIDTH) {
            i = (i + DOMINO_WIDTH) % width;
            j = j + (DOMINO_WIDTH + DOMINO_HEIGHT);
            between = false;
        } else {
            i = (i + DOMINO_WIDTH) % width;
            between = !between;
        }
    }

    let mut bytes = Vec::new();
    JpegEncoder::new(&mut bytes)
        .write_image(&img, width, height, ColorType::L8)
        .map_err(|_| "failed to encode image")?;

    Ok(ConvertResult {
        bytes,
        white_count,
        black_count,
    })
}
