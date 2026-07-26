use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::cache;
use crate::cache::CachePages;
use crate::gemini_requests::process_page;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageNode {
	url: String,
	depth: usize,
	// link was fully recursively processed
	processed: bool,
}

pub fn crawl(
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

pub fn save_graph_json(graph: &DiGraph<PageNode, ()>, path: &str) -> std::io::Result<()> {
	let json = serde_json::to_string_pretty(graph).unwrap();
	fs::write(path, json)
}

pub fn load_graph_json(path: &str) -> std::io::Result<DiGraph<PageNode, ()>> {
	let json = fs::read_to_string(path)?;
	let graph: DiGraph<PageNode, ()> = serde_json::from_str(&json).unwrap();
	Ok(graph)
}

pub fn save_graph_dot(graph: &DiGraph<PageNode, ()>, path: &str) -> std::io::Result<()> {
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
