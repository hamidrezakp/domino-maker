#[macro_use]
extern crate rocket;
extern crate image;

use rocket::data::{Data, ToByteUnit};
use rocket::http::Header;
use rocket::response::Response;

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
            white_count: Header::new("X-White-Count", white_count.to_string()),
            black_count: Header::new("X-Black-Count", black_count.to_string()),
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
fn options_handler<'a>() -> Response<'a> {
    Response::build()
        .raw_header("Access-Control-Allow-Origin", "*")
        .raw_header("Access-Control-Allow-Methods", "OPTIONS, POST")
        .raw_header("Access-Control-Allow-HEADERS", "Content-Type")
        .finilize()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![convert, options_handler])
}
