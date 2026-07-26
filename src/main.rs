use clap::Parser;
use core::hash;
use gmi::url::Path;
use gmi::{protocol::StatusCode, *};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryFrom;
use std::fs;
use std::panic;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use ::url::Url;

mod cache;
use cache::CachePages;

use crate::cache::cache_insert;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
	input: String,

	#[arg(short, long, default_value_t = "gemini://gemini.circumlunar.space/capcom".to_string())]
	start: String,

	#[arg(short, long, default_value_t = 5)]
	max_depth: usize,

	#[arg(short, long)]
	dot_path: Option<String>,

	#[arg(short, long, default_value_t = false)]
	cache: bool,

	#[arg(long, default_value = "data/cache.json")]
	cache_path: String,
}

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
struct PageNode {
	url: String,
	depth: usize,
	// link was fully recursively processed
	processed: bool,
}

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
	for link in links.iter_mut() {
    if link.starts_with("htt") {
      continue;
    }
		*link = normalize_link(link.as_str(), url);
		println!("  LINK fix: {:?}", link);
	}

	// remove all non-gemini links
	links.retain(|link| link.starts_with("gemini://"));

	Ok(links)
}

fn normalize_link(link: &str, base: &str) -> String {
	let base = match Url::try_from(base) {
		Ok(u) => u,
		Err(_) => return link.to_string(),
	};

	match base.join(link) {
		Ok(url) => url.to_string(),
		Err(_) => link.to_string(),
	}
}

fn cache_process_page(
	url: &str,
	cache: &mut CachePages,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
	if let Some(links) = cache.get(url) {
		Ok(links.clone())
	} else {
		process_page(url)
	}
}

fn crawl(
	mut graph: DiGraph<PageNode, ()>,
	start: &str,
	max_depth: usize,
	cache: &mut Option<CachePages>,
) -> DiGraph<PageNode, ()> {
	let stop = Arc::new(AtomicBool::new(false));
	let stop_handler = Arc::clone(&stop);

	ctrlc::set_handler(move || {
		println!("\nStopping");
		stop_handler.store(true, Ordering::SeqCst);
	})
	.unwrap();

	let mut indices: HashMap<String, NodeIndex> = HashMap::new();
	let mut visited: HashSet<String> = HashSet::new();

	for idx in graph.node_indices() {
		let url = graph[idx].url.clone();
		indices.insert(url.clone(), idx);
		// visited.insert(url);
	}

	let mut queue: VecDeque<(String, usize)> = VecDeque::new();
	queue.push_back((start.to_string(), 0));

	while let Some((url, depth)) = queue.pop_front() {
		if stop.load(Ordering::SeqCst) {
			break;
		}

		if visited.contains(&url) || depth > max_depth {
			continue;
		}
		visited.insert(url.clone());

		let idx = *indices.entry(url.clone()).or_insert_with(|| {
			graph.add_node(PageNode {
				url: url.clone(),
				depth,
				processed: false,
			})
		});

		if graph[idx].depth > depth {
			graph[idx].depth = depth;
		}

		let links = if let Some(c) = cache.as_mut() {
			if let Some(cached) = cache::cache_get(c, &url) {
				println!("CACHE: {}", url);
				Ok(cached)
			} else {
				match process_page(&url) {
					Ok(links) => {
						cache::cache_insert(c, &url, links.clone());
						Ok(links)
					}
					Err(e) => Err(e),
				}
			}
		} else {
			process_page(&url)
		};

		match links {
			Ok(links) => {
				for link in links {
					// skip the links that link to themselves
					if link == url {
						continue;
					}
					let link_idx = *indices.entry(link.clone()).or_insert_with(|| {
						graph.add_node(PageNode {
							url: link.clone(),
							depth,
							processed: false,
						})
					});

					if graph[link_idx].depth > depth + 1 {
						graph[link_idx].depth = depth + 1;
					}

					graph.update_edge(idx, link_idx, ());

					if !visited.contains(&link) {
						queue.push_back((link, depth + 1));
					}
				}

				graph[idx].processed = true;
			}
			Err(e) => {
				eprintln!("Failed {url}: {e}");
			}
		}
	}

	graph
}

fn save_graph_json(graph: &DiGraph<PageNode, ()>, path: &str) -> std::io::Result<()> {
	let json = serde_json::to_string_pretty(graph).unwrap();
	fs::write(path, json)
}

fn load_graph_json(path: &str) -> std::io::Result<DiGraph<PageNode, ()>> {
	let json = fs::read_to_string(path)?;
	let graph: DiGraph<PageNode, ()> = serde_json::from_str(&json).unwrap();
	Ok(graph)
}

fn save_graph_dot(graph: &DiGraph<PageNode, ()>, path: &str) -> std::io::Result<()> {
	let dot = format!(
		"{:?}",
		Dot::with_attr_getters(
			graph,
			&[Config::EdgeNoLabel],
			&|_, _| String::new(),
			&|_, (_, node)| format!("label=\"{}\"", node.url),
		)
	);
	fs::write(path, dot)
}

fn main() {
	let args = Args::parse();

	let mut cache_pages: Option<cache::CachePages> = None;

	if args.cache {
		cache_pages = match cache::load_cache(&args.cache_path) {
			Ok(c) => Some(c),
			Err(e) => Some(cache::CachePages::new()),
		};
	}

	let graph = match load_graph_json(&args.input) {
		Ok(g) => {
			println!("Loaded graph with {} nodes.", g.node_count());
			g
		}
		Err(e) => {
			eprintln!("Cannot load graph ({e}), creating new.");
			DiGraph::new()
		}
	};

	let graph = crawl(
		graph,
		"gemini://gemini.circumlunar.space/capcom",
		args.max_depth,
		&mut cache_pages,
	);

	save_graph_json(&graph, &args.input).unwrap();

	if let Some(d) = args.dot_path {
		save_graph_dot(&graph, &d).unwrap();
	}

	if let Some(cache_pages) = cache_pages.as_ref() {
		cache::save_cache(cache_pages, &args.cache_path).unwrap();
	}
}
