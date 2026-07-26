use std::collections::HashMap;
use std::fs;

pub type CachePages = HashMap<String, Vec<String>>;

pub fn cache_insert(cache: &mut CachePages, url: &str, links: Vec<String>) {
	cache.insert(url.to_string(), links);
}

pub fn cache_get(cache: &CachePages, url: &str) -> Option<Vec<String>> {
	cache.get(url).cloned()
}

pub fn save_cache(cache: &CachePages, path: &str) -> std::io::Result<()> {
	let json = serde_json::to_string_pretty(cache).unwrap();
	fs::write(path, json)
}

pub fn load_cache(path: &str) -> std::io::Result<CachePages> {
	let json = fs::read_to_string(path)?;
	let cache: CachePages = serde_json::from_str(&json)
		.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
	Ok(cache)
}

