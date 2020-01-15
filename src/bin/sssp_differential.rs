// Single source shortest path in differential dataflow

extern crate differential_dataflow;
extern crate graph_utility;
extern crate timely;

use graph_utility::parse_graph_benchmark_arguments;
use graph_utility::GraphDataGenerator;
use graph_utility::SubEventTimer;

use timely::dataflow::operators::probe::Handle;
use timely::dataflow::*;

use differential_dataflow::input::Input;
use differential_dataflow::lattice::Lattice;
use differential_dataflow::operators::*;
use differential_dataflow::Collection;

type Node = u32;
type Weight = u32;
type Edge = (Node, Node, Weight);

fn main() {
    // Parse arguments.
    let benchmark = parse_graph_benchmark_arguments(std::env::args());
    let inspect: bool = benchmark.inspect_results;
    let target = benchmark.search_query.target;
    // Start timer.
    let timer = SubEventTimer::new_timer();

    // Computation context definition.
    timely::execute_from_args(std::env::args(), move |worker| {
        let worker_index = worker.index();
        // define BFS dataflow; return handles to roots and edges inputs
        let mut probe = Handle::new();
        let (mut roots, mut graph_in) = worker.dataflow(|scope| {
            let (root_input, roots) = scope.new_collection();
            let (edge_input, graph) = scope.new_collection();
            let mut result = sssp(&graph, &roots);

            if inspect {
                result = result.filter(move |(n, _)| *n == target);
            } else {
                result = result.filter(|_| false);
            }

            result
                .map(|(_, l)| l)
                .consolidate()
                .inspect(|x| println!("Target node: {:?}", x))
                .probe_with(&mut probe);

            (root_input, edge_input)
        });
        let source = benchmark.search_query.source;
        roots.insert(source);
        roots.close();

        // Random generator engine.
        let mut gen = GraphDataGenerator::new_from_seed(10);
        if worker_index == 0 {
            timer.time_subevent("Loading", || {
                let initial_edges = gen.gen_initial_graph(&benchmark.graph_data);
                println!(
                    "Performing SSSP on {} nodes, {} edges:",
                    gen.max_num_nodes(),
                    initial_edges.len()
                );
                // Update data only on one worker.
                for edge in initial_edges.iter() {
                    graph_in.update_at(*edge, Default::default(), 1);
                }
            });
        }
        let mut initial_advance = || {
            graph_in.advance_to(1);
            graph_in.flush();
            worker.step_while(|| probe.less_than(graph_in.time()));
        };
        if worker_index == 0 {
            timer.time_subevent("Initial", initial_advance);
        } else {
            initial_advance();
        }

        let num_rounds = benchmark.num_rounds;
        for round in 0..num_rounds {
            if worker.index() == 0 {
                let batch_edges = gen.gen_graph_updates(&benchmark.graph_updates);
                // Insert elements for update
                for edge in batch_edges.into_iter() {
                    graph_in.update_at(edge, 1 + round, -1);
                }
            }
            graph_in.advance_to(2 + round);
            // Flush to input to make sure all changes are in the message queues.
            graph_in.flush();
            let mut update_advance = || {
                worker.step_while(|| probe.less_than(&graph_in.time()));
            };
            if worker_index == 0 {
                timer.time_subevent(&format!("N {}", round), update_advance);
            } else {
                update_advance();
            }
        }

        println!(
            "Worker {} finished in: {:?}",
            worker.index(),
            timer.elapsed()
        );
    })
    .unwrap();
}

fn sssp<G: Scope>(
    edges: &Collection<G, Edge>,
    roots: &Collection<G, Node>,
) -> Collection<G, (Node, Weight)>
where
    G::Timestamp: Lattice + Ord,
{
    // initialize roots as reaching themselves at distance 0
    let nodes = roots.map(|x| (x, 0));
    // Repeatedly update minimal distances each node can be reached from each root
    nodes.iterate(|inner| {
        let edges = edges
            .enter(&inner.scope())
            .map(|(from, to, w)| (from, (to, w)));
        let nodes = nodes.enter(&inner.scope());
        inner
            .join_map(&edges, |_from, &cost, &(to, w)| (to, cost + w))
            .concat(&nodes)
            // Note: reduce receives its input as an ordered collection.
            .reduce(|_, input, output| output.push((*input[0].0, 1)))
    })
}
