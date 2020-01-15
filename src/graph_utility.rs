/// File containing various utilities used in all the graph benchmarks.



/// Random number generation external libraries.
/// The generators from rand and rand_chacha are used because they are
/// reproducible on different machines.
extern crate rand;
extern crate rand_chacha;

/// Exported types representing graphs.
/// Note, these are just type aliases to tuples of elements. The reason we are doing it like so
/// is to make it a bit simple to use these types with various libraries.
/// For example, timely and differential data flow work easier with structures, while the
/// petgraph library has its own graph data structures.

pub type Node = u32;
pub type Weight = u32;

pub type UnweightedEdge = (Node, Node);
pub type WeightedEdge = (Node, Node, Weight);

/// Convenience methods for loading graphs.
/// Graph files are simply whitespace separated lists of numbers.

/// Graph loader holding the number of indexes and peers. Useful for multi-worker loading.
pub struct GraphLoader {
    index: usize,
    peers: usize,
}

impl GraphLoader {

    pub fn default() -> GraphLoader {
        GraphLoader {index: 0, peers: 1}
    }

    pub fn new(index: usize, peers: usize) -> GraphLoader {
        GraphLoader {index: index, peers: peers}
    }

    /// Load from a file containing triplets of numbers: "source target weight"
    pub fn load_weighted_graph(&self, filename: &str) -> Vec<WeightedEdge> {
        // Standard io/fs boilerplate.
        use std::io::{BufRead, BufReader};
        use std::fs::File;

        let mut data = Vec::new();
        let file = BufReader::new(File::open(filename).expect("Could open file"));
        let lines = file.lines();
        
        for (count, read_line) in lines.enumerate() {
            if count % self.peers == self.index {
                if let Ok(line) = read_line {
                    if line.starts_with("#") {
                        continue;
                    }
                    let mut text = line.split_whitespace();
                    let from = text.next().expect("Must have from node").parse().expect("Invalid from node");
                    let to = text.next().expect("Must have to node").parse().expect("Invalid to node");
                    let weight = text.next().expect("Must have node weight").parse().expect("Invalid node weight");
                    data.push((from, to, weight));
                }
            }
        }
        data
    }

    /// Load from a file containing pairs of numbers: "source target"
    pub fn load_unweighted_graph(&self, filename: &str) -> Vec<UnweightedEdge> {
        // Standard io/fs boilerplate.
        use std::io::{BufRead, BufReader};
        use std::fs::File;

        let mut data = Vec::new();
        let file = BufReader::new(File::open(filename).expect("Could open file"));
        let lines = file.lines();

        for (count, read_line) in lines.enumerate() {
            if count % self.peers == self.index {
                if let Ok(line) = read_line {
                    if line.starts_with("#") {
                        continue;
                    }
                    let mut text = line.split_whitespace();
                    let from = text.next().expect("Must have from node").parse().expect("Invalid from node");
                    let to = text.next().expect("Must have to node").parse().expect("Invalid to node");
                    data.push((from, to));
                }
            }
        }
        data
    }
}

pub use rand::SeedableRng;

pub fn default_rng(seed: u64) -> rand_chacha::ChaCha8Rng {
    rand_chacha::ChaCha8Rng::seed_from_u64(seed)
}

/// Generate a random graph with a given number of vertices and edges
pub fn generate_unweighted_graph(rng: &mut rand_chacha::ChaCha8Rng, num_nodes: u32, num_edges: u32) -> Vec<UnweightedEdge> {
    use rand::distributions::{Distribution, Uniform};

    let dist = Uniform::new(0 as Node, num_nodes as Node);
    let mut edges = Vec::new();
    for _ in 0 .. num_edges {
        let from = dist.sample(rng);
        let to = dist.sample(rng);
        edges.push((from, to));
    }
    edges
}

/// Generate a random graph with a given number of vertices, edges and weights for the edges.
pub fn generate_weighted_graph(rng: &mut rand_chacha::ChaCha8Rng, num_nodes: u32, num_edges: u32, weight_range: (Weight, Weight)) -> Vec<WeightedEdge> {
    use rand::distributions::{Distribution, Uniform};

    let dist = Uniform::new(0 as Node, num_nodes as Node);
    let dist_w = Uniform::new(weight_range.0, weight_range.1);
    let mut edges = Vec::new();
    for _ in 0 .. num_edges {
        let from = dist.sample(rng);
        let to = dist.sample(rng);
        let w = dist_w.sample(rng);
        edges.push((from, to, w));
    }
    edges
}

pub fn generate_weights_for_graph(rng: &mut rand_chacha::ChaCha8Rng, edges: Vec<UnweightedEdge>, weight_range: (Weight, Weight)) -> Vec<WeightedEdge> {
    use rand::distributions::{Distribution, Uniform};

    let dist_w = Uniform::new(weight_range.0, weight_range.1);
    edges.into_iter().map(|(from, to)| (from, to,  dist_w.sample(rng))).collect()
}

#[derive(Clone, Copy, Debug)]
enum GraphDataType {
    Random, RealWorld
}

#[derive(Clone, Copy, Debug)]
pub struct WeightParameters {
    pub weight_range: (u32, u32),
    pub rng_seed: u64,
}

#[derive(Debug)]
pub enum GraphBenchmarkData {
   RandomGraph { nodes: u32, edges: u32, weight_par: WeightParameters },
   RealWorldGraph { path_to_edge_list: String, weight_par: Option<WeightParameters> },
}

#[derive(Debug)]
pub enum GraphBenchmarkUpdates {
    RandomUpdates { edges_per_update: u32, weight_par: WeightParameters },
}

#[derive(Debug)]
pub struct SearchQuery {
    pub source: u32,
    pub target: u32,
}

#[derive(Debug)]
pub struct BenchmarkDescription {
    pub graph_data: GraphBenchmarkData,
    pub graph_updates: GraphBenchmarkUpdates,
    pub num_rounds: u32,
    pub search_query: SearchQuery,
    pub inspect_results: bool,
}

pub fn extract_weight_range(data: &GraphBenchmarkData) -> (u32, u32) {
    extract_weight_parameters(data).weight_range
}

fn extract_weight_parameters(data: &GraphBenchmarkData) -> WeightParameters {
    use GraphBenchmarkData::*;
    match data {
        RandomGraph{weight_par, ..} => *weight_par,
        RealWorldGraph{weight_par, ..} => weight_par.unwrap_or(WeightParameters{ weight_range: (0u32, 10u32), rng_seed: 10u64 }),
    }
}

/// Common command line argument parsers. Makes sure we parse the same arguments
/// in all benchmarking executables.
pub fn parse_graph_benchmark_arguments(mut arguments: std::env::Args) -> BenchmarkDescription {
    arguments.next().expect("Command line argument should contain an executable name.");

    let type_of_data: String = arguments.next().expect("Did not pass type of graph data");
    let graph_type = match type_of_data.as_str() {
        "real" => GraphDataType::RealWorld,
        "random" => GraphDataType::Random,
        _ => panic!("Invalid type of data passed. Please use one of: real, random"),
    };

    let graph_data = match graph_type {
        GraphDataType::Random => {
            let nodes: u32 = arguments.next().expect("No number of nodes passed").parse().expect("Invalid argument passed to number of nodes");
            let edges: u32 = arguments.next().expect("No number of edges passed").parse().expect("Invalid argument passed to number of edges");
            let lower_weight: u32 = arguments.next().expect("No weight lower bound passed").parse().expect("Invalid argument passed to lower bound weight");
            let upper_weight: u32 = arguments.next().expect("No weight upper bound passed").parse().expect("Invalid argument passed to upper bound weight");
            if lower_weight >= upper_weight {
                panic!("Lower weight range must be less than upper weight range");
            }
            GraphBenchmarkData::RandomGraph {nodes: nodes, edges: edges, weight_par: WeightParameters{ weight_range: (lower_weight, upper_weight), rng_seed: 10 } }
        }
        GraphDataType::RealWorld => {
            let graph_file: String = arguments.next().expect("No path to graph file given");
            let path_to_file = std::path::Path::new(&graph_file);
            if !path_to_file.exists() {
                panic!("Graph file {:?} does not exist", graph_file);
            }
            let generate_weights: bool = arguments.next().expect("No weight generation passed") == "generate";
            let weight_par  = if generate_weights {
                let lower_weight: u32 = arguments.next().expect("No weight lower bound passed").parse().expect("Invalid argument passed to lower bound weight");
                let upper_weight: u32 = arguments.next().expect("No weight upper bound passed").parse().expect("Invalid argument passed to upper bound weight");
                if lower_weight >= upper_weight {
                    panic!("Lower weight range must be less than upper weight range");
                }
                Some(WeightParameters{ weight_range: (lower_weight, upper_weight), rng_seed: 10 })
            } else {
                None
            };
            GraphBenchmarkData::RealWorldGraph { path_to_edge_list: graph_file, weight_par: weight_par }
        }
    };

    let num_rounds: u32 = arguments.next().expect("No number of rounds").parse().expect("Invalid argument passed to number of rounds");
    let edges_per_update: u32 = arguments.next().expect("No number of edges per round").parse().expect("Invalid argument passed to edges per round");

    let source: u32 = arguments.next().expect("No source node given").parse().expect("Invalid argument passed to source node");

    let target: u32 = arguments.next().expect("No target node given").parse().expect("Invalid argument passed to target node");

    let graph_updates = GraphBenchmarkUpdates::RandomUpdates{edges_per_update: edges_per_update, weight_par: extract_weight_parameters(&graph_data) };

    let search_query = SearchQuery {source: source, target: target};

    let inspect = arguments.next().map(|x| x == "inspect").unwrap_or(false);

    BenchmarkDescription{graph_data: graph_data, graph_updates: graph_updates, num_rounds: num_rounds, search_query: search_query, inspect_results: inspect}
}

pub struct GraphDataGenerator {
    rng: rand_chacha::ChaCha8Rng,
    num_nodes: u32, 
}

fn num_nodes_from_edge_list(edges: &Vec<WeightedEdge>) -> u32 {
    let mut max_node = 0;
    for (from, to, ..) in edges.iter() {
        max_node = std::cmp::max(max_node, *from);
        max_node = std::cmp::max(max_node, *to);
    }
    max_node
}

impl GraphDataGenerator {

    pub fn new_from_seed(seed: u64) -> GraphDataGenerator {
        GraphDataGenerator { rng: default_rng(seed), num_nodes: 0 }
    }

    pub fn gen_initial_graph(& mut self, desc: &GraphBenchmarkData) -> Vec<WeightedEdge> {
        use GraphBenchmarkData::*;
        match desc {
            RandomGraph {nodes, edges, weight_par} => {
                // Update the number of nodes
                self.num_nodes = *nodes;
                generate_weighted_graph(&mut self.rng, *nodes, *edges, weight_par.weight_range)
            }
            RealWorldGraph { path_to_edge_list, weight_par } => {
                let loader = GraphLoader::default();
                let edges = match &weight_par {
                    None => loader.load_weighted_graph(&path_to_edge_list),
                    Some(par) => {
                        generate_weights_for_graph(&mut self.rng, loader.load_unweighted_graph(&path_to_edge_list), par.weight_range)
                    }
                };
                self.num_nodes = num_nodes_from_edge_list(&edges);
                edges
            }
        }
    }
    
    pub fn max_num_nodes(&self) -> u32 {
        self.num_nodes
    }

    pub fn gen_graph_updates(& mut self, desc: &GraphBenchmarkUpdates) -> Vec<WeightedEdge> {
        if self.num_nodes == 0 {
            panic!("gen_graph_updates called before gen_initial_graph");
        }
        use GraphBenchmarkUpdates::*;
        let RandomUpdates{edges_per_update, weight_par, ..} = desc;
        {
            generate_weighted_graph(&mut self.rng, self.num_nodes, *edges_per_update, weight_par.weight_range)
        }
    }
}


pub struct SubEventTimer {
    total_timer: std::time::Instant,
}

impl SubEventTimer {

    pub fn new_timer() -> SubEventTimer {
        SubEventTimer { total_timer: std::time::Instant::now() }
    }

    /// Timing utilities
    pub fn time_subevent<G, F: FnMut() -> G>(&self, event: &str, mut func: F) -> G {
        let timer = std::time::Instant::now();
        let res = func();
        let elapse = timer.elapsed();
        println!("Total: {:15}{:10}{:15}", format!("{:?}", self.elapsed()), event, format!("{:?}", elapse));
        res
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.total_timer.elapsed()
    }
}
