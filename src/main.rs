use gmi::{protocol::StatusCode, *};
use std::convert::TryFrom;

#[derive(Debug)]
struct GeminiError {
	status: StatusCode,
	meta: String,
}

impl std::fmt::Display for GeminiError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}: {}", self.status, self.meta)
	}
}

impl std::error::Error for GeminiError {}

fn request_page(url: &str) -> Result<String, Box<dyn std::error::Error>> {
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
      StatusCode::PermanentFailure(c) => {
        return Err(Box::new(GeminiError { status: StatusCode::PermanentFailure(c), meta: response.meta }));
      }
			// s => return Err(format!("Unknown status code: {:?}", s).into()),
			s => return Err(format!("Unknown status code: {:?}", s).into()),
		}
	}
}

fn main() {
  	let page = match request_page("gemini://kennedy.gemi.dev/dsfg2fdwe") {
		Ok(page) => page,
		Err(e) => {
			eprintln!("Error: {e}");
			return;
		}
	};

	// println!("{page}");

	let gemtext_nodes = gemtext::parse_gemtext(page.as_str());

	println!("0: {}", &gemtext_nodes[0]);

	if let gemtext::GemtextNode::Heading(s) = &gemtext_nodes[0] {
		println!("{s}");
	} else {
		println!("Incorrect type!");
	}
}
