/// Baseline implementation using the petgraph Rust graph
/// library.

extern crate graph_utility;
extern crate petgraph;

use graph_utility::GraphDataGenerator;
use graph_utility::SubEventTimer;
use graph_utility::parse_graph_benchmark_arguments;

use petgraph::algo::bellman_ford;
use petgraph::prelude::*;
use petgraph::Graph;
use petgraph::algo::FloatMeasure;

/// Workaround as petgraph::algo::FloatMeasure is not implemented for u32 types.
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy, Default)]
struct U32(u32);

impl std::ops::Add for U32 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self (self.0 + other.0)
    }
}

impl FloatMeasure for U32 {
    fn zero() -> Self { U32(0) }
    fn infinite() -> Self { U32(u32::max_value()) }
}

fn main() {
    // Parse arguments.
    let benchmark = parse_graph_benchmark_arguments(std::env::args());

    // Start timer.
    let timer = SubEventTimer::new_timer();

    // Measure data loading.
    let graph = timer.time_subevent("Loading", ||{
        let mut gen = GraphDataGenerator::new_from_seed(10);
        // Initial graph data.
        let initial_edges = gen.gen_initial_graph(&benchmark.graph_data);
        println!(
            "Performing SSSP on {} nodes, {} edges:",
            gen.max_num_nodes(),
            initial_edges.len()
        );
        let transformed_edges : Vec<(u32, u32, f32)> = initial_edges.into_iter().map(|(to, from, w)| (to, from, w as f32)).collect();
        Graph::<(), f32, Directed, _>::from_edges(transformed_edges.into_iter())
    });
    // Random generator engine.
    let path = timer.time_subevent("Initial", || {
        let source = NodeIndex::new(benchmark.search_query.source as usize);
        bellman_ford(&graph, source)
    });
    println!(
        "petgraph Bellman-Ford algorithm finished in: {:?}",
        timer.elapsed()
    );
    let path_bare = path.expect("No negative cost cycles");
    let source = benchmark.search_query.source;
    let target = benchmark.search_query.target;
    println!("Cost from {} to {} is {}", source, target, path_bare.0[target as usize])
}
