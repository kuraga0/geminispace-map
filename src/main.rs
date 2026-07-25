use gmi::{protocol::StatusCode, *};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryFrom;
use std::fs;
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

fn request_page(url: &str) -> Result<String, Box<dyn std::error::Error>> {
	let mut url = url::Url::try_from(url)?;
	// println!("Making response with URL: {}", url);
	loop {
		let response = request::make_request(&url)?;
		match response.status {
			protocol::StatusCode::Redirect(c) => {
				println!("Redirect! Code: {} with meta {}", c, response.meta);
				url = url::Url::try_from(response.meta.as_str())?;
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
	for i in 0..links.len() {
		if links[i].starts_with('/') {
			if let Ok(parsed) = url::Url::try_from(url) {
				links[i] = format!("gemini://{}{}", parsed.authority, links[i]);
			}
		}
	}

  // remove all non-gemini links
  links.retain(|link| link.starts_with("gemini://"));

	Ok(links)
}

fn recursive_process_page(url: &str, visited: &mut HashSet<String>) {
	if visited.contains(url) {
		return;
	}
	visited.insert(url.to_string());

	println!("  Visiting: {url}");

	let links = match process_page(url) {
		Ok(links) => links,
		Err(e) => {
			eprintln!("  Failed to process {url}: {e}");
			return;
		}
	};

	for l in links {
		recursive_process_page(l.as_str(), visited);
	}
}

fn crawl(start: &str, max_depth: usize) -> DiGraph<String, ()> {
	let mut graph: DiGraph<String, ()> = DiGraph::new();
	let mut indices: HashMap<String, NodeIndex> = HashMap::new();
	let mut visited: HashSet<String> = HashSet::new();
	let mut queue: VecDeque<(String, usize)> = VecDeque::new();

	queue.push_back((start.to_string(), 0));

	while let Some((url, depth)) = queue.pop_front() {
		if visited.contains(&url) || depth > max_depth {
			continue;
		}
		visited.insert(url.clone());

		let idx = *indices
			.entry(url.clone())
			.or_insert_with(|| graph.add_node(url.clone()));

		match process_page(&url) {
			Ok(links) => {
				for link in links {
					let link_idx = *indices.entry(link.clone()).or_insert_with(|| {
						graph.add_node(link.clone())
					});

					graph.add_edge(idx, link_idx, ());

					if !visited.contains(&link) {
						queue.push_back((link, depth + 1));
					}
				}
			}
			Err(e) => {
				eprintln!("Failed {url}: {e}");
			}
		}
	}

	graph
}

fn save_graph_json(graph: &DiGraph<String, ()>, path: &str) -> std::io::Result<()> {
	let json = serde_json::to_string_pretty(graph).unwrap();
	fs::write(path, json)
}

fn load_graph_json(path: &str) -> std::io::Result<DiGraph<String, ()>> {
	let json = fs::read_to_string(path)?;
	let graph: DiGraph<String, ()> = serde_json::from_str(&json).unwrap();
	Ok(graph)
}

fn save_graph_dot(graph: &DiGraph<String, ()>, path: &str) -> std::io::Result<()> {
	let dot = format!("{:?}", Dot::with_config(graph, &[Config::EdgeNoLabel]));
	fs::write(path, dot)
}

fn main() {
	let mut graph = match load_graph_json("geminispace.json") {
		Ok(g) => g,
		Err(e) => DiGraph::new(),
	};
	graph = crawl("gemini://gemini.circumlunar.space/capcom", 15);

	save_graph_json(&graph, "geminispace.json").unwrap();

	save_graph_dot(&graph, "geminispace.dot").unwrap();
	// let mut visited = HashSet::new();
	// recursive_process_page("gemini://kennedy.gemi.dev", &mut visited);
}
