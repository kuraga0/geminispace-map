use gmi::{protocol::StatusCode, *};
use std::convert::TryFrom;
use std::collections::HashSet;

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
			s => return Err(format!("Unknown status code: {:?}", s).into()),
		}
	}
}

fn process_page(url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
	let page = match request_page(url) {
		Ok(page) => page,
		Err(e) => {
			eprintln!("Error: {e}");
			return Err(e);
		}
	};

	// println!("{page}");

	let gemtext_nodes = gemtext::parse_gemtext(page.as_str());

  let mut links: Vec<String> = Vec::new();

	for node in &gemtext_nodes {
		if let gemtext::GemtextNode::Link(link, caption) = node {
			println!("  LINK: {:?} {:?}", caption, link);
      links.push(link.to_owned());
		}
	}
  
  // fix links like "/about"
  for i in 0..links.len() {
    if links[i].starts_with("/") {
      links[i] = format!("{}{}", url, links[i]);
    }
  }

  Ok(links)
}

fn recursive_process_page(url: &str, visited: &mut HashSet<String>) {
	if visited.contains(url) {
		return;
	}
	visited.insert(url.to_string());

	println!("Visiting: {url}");

	let links = match process_page(url) {
		Ok(links) => links,
		Err(e) => {
			eprintln!("Failed to process {url}: {e}");
			return;
		}
	};

	for l in links {
		recursive_process_page(l.as_str(), visited);
	}
}

fn main() {
	let mut visited = HashSet::new();
	recursive_process_page("gemini://kennedy.gemi.dev", &mut visited);
}
