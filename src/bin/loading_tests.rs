extern crate graph_utility;

use graph_utility::parse_graph_benchmark_arguments;
use graph_utility::GraphDataGenerator;

fn main() {

    // Test arguments:
    // executable str  path      str      low high rounds update source target str?
    // load_test  real dummy.txt generate 1   10   100    5      0      1000   inspect
    //
    // executable str    nodes edges low high rounds update source target str?
    // gen_tes    random 100   100   1   20   1000   3      0      10     inspect
    
    let benchmark = parse_graph_benchmark_arguments(std::env::args());

    println!("{:?}", benchmark);

    let mut gen = GraphDataGenerator::new_from_seed(10);
    let edge_list = gen.gen_initial_graph(&benchmark.graph_data);

    for edge in edge_list.into_iter().take(100) {
        println!("Edge: {:?}", edge);
    }

}
