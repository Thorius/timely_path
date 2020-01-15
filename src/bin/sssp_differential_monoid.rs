// Single source shortest path in differential dataflow

extern crate differential_dataflow;
extern crate graph_utility;
extern crate timely;

#[macro_use]
extern crate abomonation_derive;
extern crate abomonation;
#[macro_use]
extern crate serde_derive;
extern crate serde;

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
type Edge = (Node, Node);
type Weight = u32;

#[derive(
    Abomonation, Copy, Ord, PartialOrd, Eq, PartialEq, Debug, Clone, Serialize, Deserialize, Hash,
)]
pub struct MinSum {
    value: Weight,
}

use differential_dataflow::difference::Semigroup;
use std::ops::{AddAssign, Mul};

impl<'a> AddAssign<&'a Self> for MinSum {
    fn add_assign(&mut self, rhs: &'a Self) {
        self.value = std::cmp::min(self.value, rhs.value);
    }
}

impl Mul<Self> for MinSum {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        MinSum {
            value: self.value + rhs.value,
        }
    }
}

impl Semigroup for MinSum {
    fn is_zero(&self) -> bool {
        false
    }
}

fn main() {
    // Parse arguments.
    let benchmark = parse_graph_benchmark_arguments(std::env::args());
    let inspect: bool = benchmark.inspect_results;
    let target = benchmark.search_query.target;
    // Start timer.
    let timer = SubEventTimer::new_timer();

    // Define computation graph
    timely::execute_from_args(std::env::args(), move |worker| {
        let worker_index = worker.index();

        // define BFS dataflow; return handles to roots and edges inputs
        let mut probe = Handle::new();
        let (mut roots, mut graph_in) = worker.dataflow(|scope| {
            let (root_input, roots) = scope.new_collection();
            let (edge_input, graph) = scope.new_collection();

            let mut result = sssp_monoid(&graph, &roots);

            if inspect {
                result = result.filter(move |n| *n == target);
            } else {
                result = result.filter(|_| false);
            }
            result
                .count()
                .map(|(_, l)| l)
                .consolidate()
                .inspect(|x| println!("Target node: {:?}", x))
                .probe_with(&mut probe);

            (root_input, edge_input)
        });

        let source = benchmark.search_query.source;
        roots.update_at(source, Default::default(), MinSum { value: 0 });
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
                for (from, to, w) in initial_edges.iter() {
                    graph_in.update_at((*from, *to), Default::default(), MinSum { value: *w });
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
                for (from, to, _w) in batch_edges.into_iter() {
                    graph_in.update_at((from, to), 1 + round, MinSum { value: 1000 });
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

// returns pairs (n, s) indicating node n can be reached from a root in s steps.
fn sssp_monoid<G: Scope>(
    edges: &Collection<G, Edge, MinSum>,
    roots: &Collection<G, Node, MinSum>,
) -> Collection<G, Node, MinSum>
where
    G::Timestamp: Lattice + Ord,
{
    // repeatedly update minimal distances each node can be reached from each root
    roots.scope().iterative::<u32, _, _>(|scope| {
        use differential_dataflow::operators::iterate::SemigroupVariable;
        use differential_dataflow::operators::reduce::ReduceCore;
        use differential_dataflow::trace::implementations::ord::OrdKeySpine as DefaultKeyTrace;

        use timely::order::Product;
        let variable = SemigroupVariable::new(scope, Product::new(Default::default(), 1));

        let edges = edges.enter(scope);
        let roots = roots.enter(scope);

        let result = variable
            .map(|n| (n, ()))
            .join_map(&edges, |_k, &(), d| *d)
            .concat(&roots)
            .map(|x| (x, ()))
            .reduce_core::<_, DefaultKeyTrace<_, _, _>>("Reduce", |_key, input, output, updates| {
                if output.is_empty() || input[0].1 < output[0].1 {
                    updates.push(((), input[0].1));
                }
            })
            .as_collection(|k, ()| *k);

        variable.set(&result);
        result.leave()
    })
}
