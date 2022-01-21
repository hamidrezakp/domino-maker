#[macro_use]
extern crate rocket;
extern crate image;

use rocket::{
    data::{Data, ToByteUnit},
    http::Header,
};

mod cors;
mod domino_maker;

#[derive(Responder)]
#[response(status = 200, content_type = "image/jpeg")]
struct DominoResp {
    inner: Vec<u8>,
    white_count: Header<'static>,
    black_count: Header<'static>,
}

impl DominoResp {
    pub fn new(bytes: Vec<u8>, white_count: u32, black_count: u32) -> Self {
        Self {
            inner: bytes,
            white_count: Header::new("x-white-count", white_count.to_string()),
            black_count: Header::new("x-black-count", black_count.to_string()),
        }
    }
}

#[post("/convert?<board_width>&<board_height>", data = "<data>")]
async fn convert(
    board_width: u32,
    board_height: u32,
    data: Data<'_>,
) -> Result<DominoResp, &'static str> {
    let input_bytes = data
        .open(10.megabytes())
        .into_bytes()
        .await
        .map_err(|_| "invalid input")?;

    let result = domino_maker::convert(&input_bytes, (board_width, board_height))?;
    Ok(DominoResp::new(
        result.bytes,
        result.white_count,
        result.black_count,
    ))
}

#[options("/convert")]
fn options_handler() -> &'static str {
    ""
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(cors::CORS)
        .mount("/", routes![convert, options_handler])
}
