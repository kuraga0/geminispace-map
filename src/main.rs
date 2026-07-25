use gmi::{protocol::StatusCode, *};
use std::convert::TryFrom;
use std::collections::{HashMap, HashSet, VecDeque};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::dot::{Dot, Config};
use std::fs;
use serde::{Deserialize, Serialize};
use serde_json;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PageInfo {
	url: String,
	title: Option<String>,
}

fn request_page(url: &str) -> Result<String, Box<dyn std::error::Error>> {
	let mut url = url::Url::try_from(url).unwrap();
	// println!("Making response with URL: {}", url);
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
			// eprintln!("Error: {e}");
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
    if links[i].starts_with('/') {
      if let Ok(parsed) = url::Url::try_from(url) {
        links[i] = format!("gemini://{}{}", parsed.authority, links[i]);
      }
    }
  }

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

fn crawl(start: &str, max_depth: usize) -> DiGraph<PageInfo, ()> {
	let mut graph: DiGraph<PageInfo, ()> = DiGraph::new();
	let mut indices: HashMap<String, NodeIndex> = HashMap::new();
	let mut visited: HashSet<String> = HashSet::new();
	let mut queue: VecDeque<(String, usize)> = VecDeque::new();

	queue.push_back((start.to_string(), 0));

	while let Some((url, depth)) = queue.pop_front() {
		if visited.contains(&url) || depth > max_depth {
			continue;
		}
		visited.insert(url.clone());

		// добавляем узел, если ещё не добавлен
		let idx = *indices.entry(url.clone()).or_insert_with(|| {
			graph.add_node(PageInfo { url: url.clone(), title: None })
		});

		match process_page(&url) {
			Ok(links) => {
				for link in links {
					let link_idx = *indices.entry(link.clone()).or_insert_with(|| {
						graph.add_node(PageInfo { url: link.clone(), title: None })
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

fn save_graph_json(graph: &DiGraph<PageInfo, ()>, path: &str) -> std::io::Result<()> {
	let json = serde_json::to_string_pretty(graph).unwrap();
	fs::write(path, json)
}

fn load_graph_json(path: &str) -> std::io::Result<DiGraph<PageInfo, ()>> {
	let json = fs::read_to_string(path)?;
	let graph: DiGraph<PageInfo, ()> = serde_json::from_str(&json).unwrap();
	Ok(graph)
}

fn save_graph_dot(graph: &DiGraph<PageInfo, ()>, path: &str) -> std::io::Result<()> {
	let dot = format!("{:?}", Dot::with_config(graph, &[Config::EdgeNoLabel]));
	fs::write(path, dot)
}

fn main() {
	let mut graph = match load_graph_json("geminispace.json") {
		Ok(g) => g,
		Err(e) => {
			DiGraph::new()
		}
	};
  graph = crawl("gemini://gemini.circumlunar.space/capcom", 1);

  save_graph_json(&graph, "geminispace.json").unwrap();

  // dot_export_graph(&graph, "geminispace.dot").unwrap();
	// let mut visited = HashSet::new();
	// recursive_process_page("gemini://kennedy.gemi.dev", &mut visited);
}
