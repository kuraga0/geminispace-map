use gmi::*;
use std::convert::TryFrom;

fn request_page(url: &str) -> Result<String, request::RequestError> {
	let mut url = url::Url::try_from(url).unwrap();
	println!("Making response with URL: {}", url);
	loop {
		let response = request::make_request(&url)?;
		match response.status {
			protocol::StatusCode::Redirect(c) => {
				println!("Redirect! Code: {} with meta {}", c, response.meta);
				url = url::Url::try_from(response.meta.as_str()).unwrap();
				println!("New URL: {}", url);
			}
			protocol::StatusCode::Success(c) => {
				println!("Success! Code: {} with MIME type: {}", c, response.meta);
				return Ok(String::from_utf8_lossy(&response.data).into_owned());
			}
			s => panic!("Unknown status code: {:?}", s),
		}
	}
}

fn main() {
  let page = request_page("gemini://station.martinrue.com").unwrap();

  println!("{page}");
}
