use crate::link::normalize_link;
use gmi::{protocol::StatusCode, *};
use std::convert::TryFrom;
use std::panic;

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

fn parse_gemtext_safe(page: &str) -> Option<Vec<gemtext::GemtextNode>> {
	panic::catch_unwind(|| gemtext::parse_gemtext(page)).ok()
}

pub fn request_page(url: &str) -> Result<String, Box<dyn std::error::Error>> {
	let mut url = url::Url::try_from(url)?;
	// println!("Making response with URL: {}", url);
	loop {
		let response = request::make_request(&url)?;
		match response.status {
			protocol::StatusCode::Redirect(c) => {
				println!("  Redirect! Code: {} with meta {}", c, response.meta);
				url = url::Url::try_from(response.meta.as_str())?;
				println!("  New URL: {}", url);
			}
			protocol::StatusCode::Success(_) => {
				// println!("Success! Code: {} with MIME type: {}", c, response.meta);
				return Ok(String::from_utf8_lossy(&response.data).into_owned());
			}
			StatusCode::PermanentFailure(c) => {
				return Err(Box::new(GeminiError {
					status: StatusCode::PermanentFailure(c),
					meta: response.meta,
				}));
			}
			// s => return Err(format!("Unknown status code: {:?}", s).into()),
			s => return Err(format!("  Unknown status code: {:?}", s).into()),
		}
	}
}

pub fn process_page(url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
	let page = match request_page(url) {
		Ok(page) => page,
		Err(e) => {
			// eprintln!("Error: {e}");
			return Err(e);
		}
	};

	// println!("{page}");

	let gemtext_nodes = match parse_gemtext_safe(page.as_str()) {
		Some(nodes) => nodes,
		None => {
			eprintln!("  Skipping {} because error occured when parsing gmi.", url);
			return Ok(Vec::new());
		}
	};

	let mut links: Vec<String> = Vec::new();

	for node in &gemtext_nodes {
		if let gemtext::GemtextNode::Link(link, caption) = node {
			println!("  LINK: {:?} {:?}", caption, link);
			links.push(link.to_owned());
		}
	}

	// fix links like "/about"
	for link in links.iter_mut() {
		*link = normalize_link(link.as_str(), url);
		println!("  LINK fix: {:?}", link);
	}

	// remove all non-gemini links
	links.retain(|link| link.starts_with("gemini://"));

	const SKIP_EXTENSIONS: &[&str] = &[
		".mp3", ".opus", ".wav", ".flac", ".mp4", ".png", ".webp", ".jpeg", ".jpg", ".exe", ".fontpack",
	];
	links.retain(|link| !SKIP_EXTENSIONS.iter().any(|suf| link.ends_with(suf)));

	Ok(links)
}
