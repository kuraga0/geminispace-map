use clap::Parser;
use petgraph::graph::DiGraph;

mod cache;
mod gemini_requests;
mod graph;
mod link;

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

fn main() {
	let args = Args::parse();

	let mut cache_pages: Option<cache::CachePages> = None;

	if args.cache {
		cache_pages = match cache::load_cache(&args.cache_path) {
			Ok(c) => Some(c),
			Err(_) => Some(cache::CachePages::new()),
		};
	}

	let graph = match graph::load_graph_json(&args.input) {
		Ok(g) => {
			println!("Loaded graph with {} nodes.", g.node_count());
			g
		}
		Err(e) => {
			eprintln!("Cannot load graph ({e}), creating new.");
			DiGraph::new()
		}
	};

	let graph = graph::crawl(
		graph,
		args.start.as_str(),
		args.max_depth,
		&mut cache_pages,
	);

	graph::save_graph_json(&graph, &args.input).unwrap();

	if let Some(d) = args.dot_path {
		graph::save_graph_dot(&graph, &d).unwrap();
	}

	if let Some(cache_pages) = &cache_pages {
		cache::save_cache(cache_pages, &args.cache_path).unwrap();
	}
}
