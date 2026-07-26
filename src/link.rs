use url::Url;

pub const GEMINI_DEFAULT_PORT: u16 = 1965;

/// normalize a gemini link relative to the base
/// can use as a graph/cache key
pub fn normalize_link(link: &str, base: &str) -> String {
	let base = match Url::parse(base) {
		Ok(u) => u,
		Err(_) => return link.to_string(),
	};

	let mut url = match base.join(link) {
		Ok(u) => u,
		Err(_) => return link.to_string(),
	};

	normalize_url(&mut url);
	url.to_string()
}

fn normalize_url(url: &mut Url) {
	if url.port() == Some(GEMINI_DEFAULT_PORT) {
		let _ = url.set_port(None);
	}

	url.set_fragment(None);

	if let Some(host) = url.host_str() {
		let lower = host.to_ascii_lowercase();
		url.set_host(Some(&lower)).unwrap();
	}

	if url.path().is_empty() {
		url.set_path("/");
	}
}
