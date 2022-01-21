use image::{
    imageops::FilterType, io::Reader as ImageReader, jpeg::JpegEncoder, ColorType, GenericImage,
    ImageBuffer, ImageEncoder, Luma, Pixel,
};
use std::fmt;
use std::io::Cursor;
use std::ops::Add;

const DOMINO_WIDTH: u32 = 8;
const DOMINO_HEIGHT: u32 = 24;

pub struct ConvertResult {
    pub bytes: Vec<u8>,
    pub map: Vec<Vec<String>>,
    pub white_count: u32,
    pub black_count: u32,
}

#[derive(Copy, Clone)]
enum Domino {
    Black(u32),
    White(u32),
}

impl Domino {
    pub fn color(&self) -> u8 {
        match self {
            Domino::Black(_) => 0x0,
            Domino::White(_) => 0xff,
        }
    }
}

impl fmt::Display for Domino {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Domino::Black(c) => format!("{}b", c),
            Domino::White(c) => format!("{}w", c),
        };
        write!(f, "{}", s)
    }
}

impl Add for Domino {
    type Output = (Self, Option<Self>);

    fn add(self, other: Self) -> Self::Output {
        match (self, other) {
            (Domino::Black(fc), Domino::Black(sc)) => (Domino::Black(fc + sc), None),
            (Domino::White(fc), Domino::White(sc)) => (Domino::White(fc + sc), None),
            (Domino::Black(fc), Domino::White(sc)) => (Domino::Black(fc), Some(Domino::White(sc))),
            (Domino::White(fc), Domino::Black(sc)) => (Domino::White(fc), Some(Domino::Black(sc))),
        }
    }
}

fn get_final_image_size(board_size: (u32, u32)) -> (u32, u32) {
    let width = DOMINO_WIDTH * (2 * board_size.0 - 1);
    let height = DOMINO_WIDTH * (4 * board_size.1 - 1);
    (width, height)
}

fn simplify_image(img: &mut ImageBuffer<Luma<u8>, Vec<u8>>) {
    img.inner_mut()
        .pixels_mut()
        .for_each(|i| i.apply(|j| if j < 128 { 0 } else { 0xff }));
}

fn encode_to_jpg(
    image: &ImageBuffer<Luma<u8>, Vec<u8>>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, &'static str> {
    let mut bytes = Vec::new();
    JpegEncoder::new(&mut bytes)
        .write_image(image, width, height, ColorType::L8)
        .map_err(|_| "failed to encode image")?;
    Ok(bytes)
}

fn init_and_resize_image(
    raw_image: &[u8],
    board_size: (u32, u32),
) -> Result<(ImageBuffer<Luma<u8>, Vec<u8>>, u32, u32), &'static str> {
    let img = ImageReader::new(Cursor::new(raw_image))
        .with_guessed_format()
        .map_err(|_| "invalid bytes")?
        .decode()
        .map_err(|_| "unknown image type")?;

    let (width, height) = get_final_image_size(board_size);

    let mut img = img
        .resize_exact(width, height, FilterType::Nearest)
        .to_luma8();

    simplify_image(&mut img);
    Ok((img, width, height))
}
pub fn convert(bytes: &[u8], board_size: (u32, u32)) -> Result<ConvertResult, &'static str> {
    let (mut img, width, height) = init_and_resize_image(bytes, board_size)?;

    let mut i = 0;
    let mut j = 0;
    let mut between = false;

    let mut domino_map = Vec::new();
    let mut row_map = Vec::new();

    while j < height {
        let sum = if between {
            None
        } else {
            let mut sum = 0;
            for x in 0..DOMINO_WIDTH {
                for y in 0..DOMINO_HEIGHT {
                    sum = sum + (img.get_pixel(i + x, j + y).channels()[0]) as u32;
                }
            }
            Some(sum)
        };

        let domino = sum.map(|s| s / DOMINO_WIDTH * DOMINO_HEIGHT).map(|color| {
            if color < 128 {
                Domino::Black(1)
            } else {
                Domino::White(1)
            }
        });

        // Add domino to row map
        match (domino, row_map.pop()) {
            (None, _) => (),
            (Some(domino), None) => row_map.push(domino),
            (Some(domino), Some(d)) => match d + domino {
                (result, None) => row_map.push(result),
                (result, Some(new)) => {
                    row_map.push(result);
                    row_map.push(new);
                }
            },
        };

        // Colorize domino area
        for x in 0..DOMINO_WIDTH {
            for y in 0..DOMINO_HEIGHT {
                img.get_pixel_mut(i + x, j + y)
                    .apply(|_| domino.map(|d| d.color()).unwrap_or(0xee));
            }
        }

        // Make white square below domino
        if j != (height - DOMINO_HEIGHT) {
            for x in 0..DOMINO_WIDTH {
                for y in 0..DOMINO_WIDTH {
                    img.get_pixel_mut(i + x, j + DOMINO_HEIGHT + y)
                        .apply(|_| 0xee);
                }
            }
        }

        if i == (width - DOMINO_WIDTH) {
            i = 0;
            j = j + (DOMINO_WIDTH + DOMINO_HEIGHT);
            between = false;
            domino_map.push(row_map.clone());
            row_map.clear();
        } else {
            i = i + DOMINO_WIDTH;
            between = !between;
        }
    }

    let jpg_image = encode_to_jpg(&img, width, height)?;

    let (white_count, black_count) =
        domino_map
            .concat()
            .iter()
            .fold((0, 0), |(bc, wc), d| match d {
                Domino::Black(c) => (bc + c, wc),
                Domino::White(c) => (bc, wc + c),
            });

    let domino_map = domino_map
        .iter()
        .map(|row| row.iter().map(|d| d.to_string()).collect())
        .collect();

    Ok(ConvertResult {
        bytes: jpg_image,
        map: domino_map,
        white_count,
        black_count,
    })
}
