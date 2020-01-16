# Timely Path

Prototype implementation of graph path problems using [timely dataflow](https://github.com/timelydataflow/timely-dataflow) [differential dataflow](https://github.com/timelydataflow/differential-dataflow).

## Overview

The project uses the uses the conventional Cargo [package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html). Each benchmark is in a separate file in the _src/bin_ directory. All utility files are directly in the _src_ directory. The directory _data_ contains several road network specification. The CA, PA and TX networks are from Stanford Large Network Dataset Collection ([SNAP](https://snap.stanford.edu/data/#road)).

## Running the Benchmarks

[Install Rust](https://www.rust-lang.org/learn/get-started) and to build all benchmarks with optimizations run:

```bash
cargo build --release
```

Each benchmark is executed with the following command:

```bash
cargo run --release --bin <benchmark_name> -- <benchmark_args> <timely_args>
```

where `<benchmark_name>` is the name of a file in the _src\bin_ directory without the extension, `<benchmark_args>` the benchmark arguments explained in the following subsection and `<timely_args>`
are extra parameters to pass to the timely dataflow runtime.

### Benchmark arguments

Each benchmark can use either externally loaded data or randomly generated data. Here are the required parameters for both cases:

* External data: `<benchmark_args> := real <path_to_file> <generate_string> <low> <high> <rounds> <per_update> <source> <target> <inspect_string>`
  * `<path_to_file>`: Path to a text file with list of edges described as pairs of nodes. See the _data\roadNet-dummy.txt_ file for format specification.
  * `<generate_string>`: If the graph does not contain edge weights, this can contain the string `generate`. If this is any other string, skip the next two parameters, `<low>` and `<high>`.
* Generated data: `<benchmark_args := random <nodes> <edges> <low> <high> <rounds> <per_update> <source> <target> <inspect_string>`
  * `<nodes>`: Integer for the number of nodes in the generated graph.
  * `<edges>`: Interger for the number of edges in the generated graph
* Common parameters
  * `<low> <high>`: Two integers specifying the range for generating weights for each edge.
  * `<rounds>`: Integer specifying how many rounds of updates to do after the initial path solution is found.
  * `<per_update>`: Number of edges to augment per round.
  * `<source> <target>`: Node indices specifying the beginning and end of the searched for path.
  * `<inspect_string>`: If passed the `inspect` string, show the result of the calculation. If any other string is passed, only timing information will be printed.

### Timely Arguments

Any extra arguments will be used by timely dataflow. The primary arguments of interest is the number of workers parameter `-w <N>` where `<N>` is an integer.

### Examples

```cargo run --release --bin sssp_differential real dummy.txt generate 1 10 100 5 0 1000 inspect```

Run the sssp_differential benchmark with a graph from the dummy.txt with randomly generated weights between 1 and 10, doing 100 rounds of graph updates each with with 5 edges. Search for the best path between nodes 0 and 1000. Print the final result.

```cargo run --release --bin sssp_differential random 100 300 1 20 1000 3 0 10 no -w6```

Run the sssp_differential_monoid benchmark with a randomly generated graph with 100 nodes and 300 edges with weights between 1 and 20, doing 1000 rounds of graph updates each with with 3 edges. Search for the best path between nodes 0 and 10. Do not print the final result. Run with 6 timely workers.
