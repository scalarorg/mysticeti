// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    fs::{self, File},
    hash::Hash,
    io::{BufWriter, Write},
    marker::PhantomData,
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use crate::faults::FaultsType;

use chrono::{DateTime, Utc};
use prettytable::{Cell, Row, Table};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::measurement::MeasurementsCollection;

pub trait BenchmarkType:
    Serialize
    + DeserializeOwned
    + Default
    + Clone
    + FromStr
    + Display
    + Debug
    + PartialEq
    + Eq
    + Hash
    + PartialOrd
    + Ord
    + FromStr
{
}

/// The benchmark parameters for a run.
#[derive(Serialize, Deserialize, Clone)]
pub struct BenchmarkParameters<T> {
    /// The type of benchmark to run.
    pub benchmark_type: T,
    /// The committee size.
    pub nodes: usize,
    /// The number of (crash-)faults.
    pub faults: FaultsType,
    /// The total load (tx/s) to submit to the system.
    pub load: usize,
    /// The duration of the benchmark.
    pub duration: Duration,
}

impl<T: BenchmarkType> Default for BenchmarkParameters<T> {
    fn default() -> Self {
        Self {
            benchmark_type: T::default(),
            nodes: 4,
            faults: FaultsType::default(),
            load: 500,
            duration: Duration::from_secs(60),
        }
    }
}

impl<T: BenchmarkType> Debug for BenchmarkParameters<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}-{:?}-{}-{}",
            self.benchmark_type, self.faults, self.nodes, self.load
        )
    }
}

impl<T> Display for BenchmarkParameters<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} nodes ({}) - {} tx/s",
            self.nodes, self.faults, self.load
        )
    }
}

impl<T> BenchmarkParameters<T> {
    /// Make a new benchmark parameters.
    pub fn new(
        benchmark_type: T,
        nodes: usize,
        faults: FaultsType,
        load: usize,
        duration: Duration,
    ) -> Self {
        Self {
            benchmark_type,
            nodes,
            faults,
            load,
            duration,
        }
    }
}

/// The load type to submit to the nodes.
pub enum LoadType {
    /// Submit a fixed set of loads (one per benchmark run).
    Fixed(Vec<usize>),

    /// Search for the breaking point of the L-graph.
    // TODO: Doesn't work very well, use tps regression as additional signal.
    #[allow(dead_code)]
    Search {
        /// The initial load to test (and use a baseline).
        starting_load: usize,
        /// The maximum number of iterations before converging on a breaking point.
        max_iterations: usize,
    },
}

/// Generate benchmark parameters (one set of parameters per run).
// TODO: The rusty thing to do would be to implement Iter.
pub struct BenchmarkParametersGenerator<T: BenchmarkType> {
    /// The type of benchmark to run.
    benchmark_type: T,
    /// The committee size.
    pub nodes: usize,
    /// The load type.
    load_type: LoadType,
    /// The number of faulty nodes.
    pub faults: FaultsType,
    /// The duration of the benchmark.
    duration: Duration,
    /// The load of the next benchmark run.
    next_load: Option<usize>,
    /// Temporary hold a lower bound of the breaking point.
    lower_bound_result: Option<MeasurementsCollection<T>>,
    /// Temporary hold an upper bound of the breaking point.
    upper_bound_result: Option<MeasurementsCollection<T>>,
    /// The current number of iterations.
    iterations: usize,
}

impl<T: BenchmarkType> Iterator for BenchmarkParametersGenerator<T> {
    type Item = BenchmarkParameters<T>;

    /// Return the next set of benchmark parameters to run.
    fn next(&mut self) -> Option<Self::Item> {
        self.next_load.map(|load| {
            BenchmarkParameters::new(
                self.benchmark_type.clone(),
                self.nodes,
                self.faults.clone(),
                load,
                self.duration,
            )
        })
    }
}

impl<T: BenchmarkType> BenchmarkParametersGenerator<T> {
    /// The default benchmark duration.
    const DEFAULT_DURATION: Duration = Duration::from_secs(180);

    /// make a new generator.
    pub fn new(nodes: usize, mut load_type: LoadType) -> Self {
        let next_load = match &mut load_type {
            LoadType::Fixed(loads) => {
                if loads.is_empty() {
                    None
                } else {
                    Some(loads.remove(0))
                }
            }
            LoadType::Search { starting_load, .. } => Some(*starting_load),
        };
        Self {
            benchmark_type: T::default(),
            nodes,
            load_type,
            faults: FaultsType::default(),
            duration: Self::DEFAULT_DURATION,
            next_load,
            lower_bound_result: None,
            upper_bound_result: None,
            iterations: 0,
        }
    }

    /// Set the benchmark type.
    pub fn with_benchmark_type(mut self, benchmark_type: T) -> Self {
        self.benchmark_type = benchmark_type;
        self
    }

    /// Set crash-recovery pattern and the number of faulty nodes.
    pub fn with_faults(mut self, faults: FaultsType) -> Self {
        self.faults = faults;
        self
    }

    /// Set a custom benchmark duration.
    pub fn with_custom_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Detects whether the latest benchmark parameters run the system out of capacity.
    fn out_of_capacity(
        last_result: &MeasurementsCollection<T>,
        new_result: &MeasurementsCollection<T>,
    ) -> bool {
        let Some(first_label) = new_result.labels().next() else {
            return false;
        };

        // We consider the system is out of capacity if the latency increased by over 5x with
        // respect to the latest run.
        let threshold = last_result.aggregate_average_latency(first_label) * 5;
        let high_latency = new_result.aggregate_average_latency(first_label) > threshold;

        // Or if the throughput is less than 2/3 of the input rate.
        let last_load = new_result.transaction_load() as u64;
        let no_throughput_increase = new_result.aggregate_tps(first_label) < (2 * last_load / 3);

        high_latency || no_throughput_increase
    }

    /// Register a new benchmark measurements collection. These results are used to determine
    /// whether the system reached its breaking point.
    pub fn register_result(&mut self, result: MeasurementsCollection<T>) {
        self.next_load = match &mut self.load_type {
            LoadType::Fixed(loads) => {
                if loads.is_empty() {
                    None
                } else {
                    Some(loads.remove(0))
                }
            }
            LoadType::Search { max_iterations, .. } => {
                // Terminate the the search.
                if self.iterations >= *max_iterations {
                    None

                // Search for the breaking point.
                } else {
                    self.iterations += 1;
                    match (&mut self.lower_bound_result, &mut self.upper_bound_result) {
                        (None, None) => {
                            let next = result.transaction_load() * 2;
                            self.lower_bound_result = Some(result);
                            Some(next)
                        }
                        (Some(lower), None) => {
                            if Self::out_of_capacity(lower, &result) {
                                let next =
                                    (lower.transaction_load() + result.transaction_load()) / 2;
                                self.upper_bound_result = Some(result);
                                Some(next)
                            } else {
                                let next = result.transaction_load() * 2;
                                *lower = result;
                                Some(next)
                            }
                        }
                        (Some(lower), Some(upper)) => {
                            if Self::out_of_capacity(lower, &result) {
                                *upper = result;
                            } else {
                                *lower = result;
                            }
                            Some((lower.transaction_load() + upper.transaction_load()) / 2)
                        }
                        _ => panic!("Benchmark parameters generator is in an incoherent state"),
                    }
                }
            }
        };
    }
}

/// Network type for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkType {
    Local,
    Remote,
}

/// Comprehensive benchmark result structure
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkResult<T: BenchmarkType + DeserializeOwned> {
    /// Network type (local or remote)
    pub network_type: NetworkType,
    /// Benchmark parameters
    pub parameters: BenchmarkParameters<T>,
    /// Measurement collection
    pub measurements: MeasurementsCollection<T>,
    /// Timestamp when benchmark was completed
    pub timestamp: DateTime<Utc>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl<T: BenchmarkType + DeserializeOwned> BenchmarkResult<T> {
    pub fn new(
        network_type: NetworkType,
        parameters: BenchmarkParameters<T>,
        measurements: MeasurementsCollection<T>,
    ) -> Self {
        Self {
            network_type,
            parameters,
            measurements,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Print benchmark results to console
    pub fn print_to_console(&self) {
        println!("\n{}", "=".repeat(80));
        println!("BENCHMARK RESULTS");
        println!("{}", "=".repeat(80));
        println!("Network Type: {:?}", self.network_type);
        println!("Timestamp: {}", self.timestamp);
        println!("Parameters: {:?}", self.parameters);
        println!("Duration: {:?}", self.parameters.duration);

        // Print summary table
        self.print_summary_table();

        // Print detailed metrics
        self.print_detailed_metrics();

        println!("{}", "=".repeat(80));
    }

    /// Print summary table
    fn print_summary_table(&self) {
        let mut table = Table::new();
        table.add_row(Row::new(vec![
            Cell::new("Metric"),
            Cell::new("Value"),
            Cell::new("Unit"),
        ]));

        if let Some(label) = self.measurements.labels().next() {
            let tps = self.measurements.aggregate_tps(label);
            let avg_latency = self.measurements.aggregate_average_latency(label);
            let stdev_latency = self.measurements.aggregate_stdev_latency(label);
            let transaction_load = self.parameters.load;

            table.add_row(Row::new(vec![
                Cell::new("Throughput"),
                Cell::new(&format!("{}", tps)),
                Cell::new("tx/s"),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Average Latency"),
                Cell::new(&format!("{:.2}", avg_latency.as_millis())),
                Cell::new("ms"),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Latency Std Dev"),
                Cell::new(&format!("{:.2}", stdev_latency.as_millis())),
                Cell::new("ms"),
            ]));
            table.add_row(Row::new(vec![
                Cell::new("Input Load"),
                Cell::new(&format!("{}", transaction_load)),
                Cell::new("tx/s"),
            ]));
        }

        table.printstd();
        println!();
    }

    /// Print detailed metrics
    fn print_detailed_metrics(&self) {
        println!("DETAILED METRICS:");
        println!("{}", "-".repeat(40));

        for label in self.measurements.labels() {
            println!("Label: {}", label);
            let tps = self.measurements.aggregate_tps(label);
            let avg_latency = self.measurements.aggregate_average_latency(label);
            let stdev_latency = self.measurements.aggregate_stdev_latency(label);

            println!("  Throughput: {} tx/s", tps);
            println!("  Average Latency: {:.2} ms", avg_latency.as_millis());
            println!("  Latency Std Dev: {:.2} ms", stdev_latency.as_millis());
            println!();
        }
    }

    /// Save benchmark results to file
    pub fn save_to_file(&self, output_dir: &PathBuf) -> std::io::Result<()> {
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;

        // Generate filename based on timestamp and network type
        let timestamp_str = self.timestamp.format("%Y%m%d_%H%M%S");
        let network_str = match self.network_type {
            NetworkType::Local => "local",
            NetworkType::Remote => "remote",
        };

        let filename = format!(
            "benchmark_{}_{}_{}nodes_{}txs.json",
            network_str, timestamp_str, self.parameters.nodes, self.parameters.load
        );
        let filepath = output_dir.join(filename);

        // Save as JSON
        let file = File::create(&filepath)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;

        println!("Benchmark results saved to: {}", filepath.display());

        // Also save a human-readable summary
        let summary_filename = format!(
            "benchmark_{}_{}_{}nodes_{}txs_summary.txt",
            network_str, timestamp_str, self.parameters.nodes, self.parameters.load
        );
        let summary_filepath = output_dir.join(summary_filename);

        let mut summary_file = File::create(&summary_filepath)?;
        writeln!(summary_file, "BENCHMARK SUMMARY")?;
        writeln!(summary_file, "{}", "=".repeat(50))?;
        writeln!(summary_file, "Network Type: {:?}", self.network_type)?;
        writeln!(summary_file, "Timestamp: {}", self.timestamp)?;
        writeln!(summary_file, "Parameters: {:?}", self.parameters)?;
        writeln!(summary_file, "Duration: {:?}", self.parameters.duration)?;
        writeln!(summary_file)?;

        if let Some(label) = self.measurements.labels().next() {
            let tps = self.measurements.aggregate_tps(label);
            let avg_latency = self.measurements.aggregate_average_latency(label);
            let stdev_latency = self.measurements.aggregate_stdev_latency(label);

            writeln!(summary_file, "SUMMARY METRICS:")?;
            writeln!(summary_file, "Throughput: {} tx/s", tps)?;
            writeln!(
                summary_file,
                "Average Latency: {:.2} ms",
                avg_latency.as_millis()
            )?;
            writeln!(
                summary_file,
                "Latency Std Dev: {:.2} ms",
                stdev_latency.as_millis()
            )?;
            writeln!(summary_file, "Input Load: {} tx/s", self.parameters.load)?;
        }

        writeln!(summary_file, "{}", "=".repeat(50))?;

        Ok(())
    }
}

/// Comprehensive benchmark runner that supports both local and remote networks
pub struct BenchmarkRunner<T: BenchmarkType + DeserializeOwned> {
    /// Output directory for benchmark results
    output_dir: PathBuf,
    /// Whether to print results to console
    console_output: bool,
    /// Whether to save results to file
    file_output: bool,
    /// Phantom data for type parameter
    _phantom: PhantomData<T>,
}

impl<T: BenchmarkType + DeserializeOwned> BenchmarkRunner<T> {
    /// Create a new benchmark runner
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            output_dir,
            console_output: true,
            file_output: true,
            _phantom: PhantomData,
        }
    }

    /// Set console output flag
    pub fn with_console_output(mut self, enabled: bool) -> Self {
        self.console_output = enabled;
        self
    }

    /// Set file output flag
    pub fn with_file_output(mut self, enabled: bool) -> Self {
        self.file_output = enabled;
        self
    }

    /// Run benchmarks for both local and remote networks
    pub async fn run_comprehensive_benchmarks(
        &self,
        local_generator: BenchmarkParametersGenerator<T>,
        remote_generator: BenchmarkParametersGenerator<T>,
    ) -> Result<Vec<BenchmarkResult<T>>, Box<dyn std::error::Error>> {
        let mut all_results = Vec::new();

        // Run local network benchmarks
        println!("Starting LOCAL network benchmarks...");
        let local_results = self
            .run_network_benchmarks(NetworkType::Local, local_generator)
            .await?;
        all_results.extend(local_results);

        // Run remote network benchmarks
        println!("Starting REMOTE network benchmarks...");
        let remote_results = self
            .run_network_benchmarks(NetworkType::Remote, remote_generator)
            .await?;
        all_results.extend(remote_results);

        // Print comprehensive summary
        self.print_comprehensive_summary(&all_results);

        Ok(all_results)
    }

    /// Run benchmarks for a specific network type
    async fn run_network_benchmarks(
        &self,
        network_type: NetworkType,
        mut generator: BenchmarkParametersGenerator<T>,
    ) -> Result<Vec<BenchmarkResult<T>>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        let mut benchmark_count = 1;

        while let Some(parameters) = generator.next() {
            println!(
                "\nRunning {:?} benchmark {}: {:?}",
                network_type, benchmark_count, parameters
            );

            // Here you would integrate with the existing orchestrator
            // For now, we'll create a mock result
            let measurements = self.run_single_benchmark(&parameters).await?;

            let result =
                BenchmarkResult::new(network_type.clone(), parameters, measurements.clone());

            // Output results
            if self.console_output {
                result.print_to_console();
            }

            if self.file_output {
                result.save_to_file(&self.output_dir)?;
            }

            results.push(result);
            generator.register_result(measurements);
            benchmark_count += 1;
        }

        Ok(results)
    }

    /// Run a single benchmark (placeholder - integrate with existing orchestrator)
    async fn run_single_benchmark(
        &self,
        parameters: &BenchmarkParameters<T>,
    ) -> Result<MeasurementsCollection<T>, Box<dyn std::error::Error>> {
        // TODO: Integrate with existing orchestrator
        // For now, return a mock measurement collection
        use crate::settings::Settings;

        // Create a mock settings for testing
        let settings = Settings {
            testbed_id: "test".to_string(),
            cloud_provider: crate::settings::CloudProvider::Aws,
            token_file: PathBuf::from("test"),
            ssh_private_key_file: PathBuf::from("test"),
            ssh_public_key_file: None,
            regions: vec!["us-west-1".to_string()],
            specs: "t3.medium".to_string(),
            repository: crate::settings::Repository {
                url: reqwest::Url::parse("https://github.com/test/test").unwrap(),
                commit: "test".to_string(),
            },
            working_dir: PathBuf::from("test"),
            results_dir: PathBuf::from("test"),
            logs_dir: PathBuf::from("test"),
        };

        let mut collection = MeasurementsCollection::new(&settings, parameters.clone());

        // Add some mock data
        let (label, measurement) = crate::measurement::Measurement::new_for_test();
        collection.add(1, label, measurement);

        Ok(collection)
    }

    /// Print comprehensive summary of all benchmark results
    fn print_comprehensive_summary(&self, results: &[BenchmarkResult<T>]) {
        println!("\n{}", "=".repeat(80));
        println!("COMPREHENSIVE BENCHMARK SUMMARY");
        println!("{}", "=".repeat(80));

        // Group results by network type
        let mut local_results = Vec::new();
        let mut remote_results = Vec::new();

        for result in results {
            match result.network_type {
                NetworkType::Local => local_results.push(result),
                NetworkType::Remote => remote_results.push(result),
            }
        }

        // Print local network summary
        if !local_results.is_empty() {
            println!("\nLOCAL NETWORK RESULTS:");
            println!("{}", "-".repeat(40));
            self.print_network_summary(&local_results);
        }

        // Print remote network summary
        if !remote_results.is_empty() {
            println!("\nREMOTE NETWORK RESULTS:");
            println!("{}", "-".repeat(40));
            self.print_network_summary(&remote_results);
        }

        // Print comparison if both types exist
        if !local_results.is_empty() && !remote_results.is_empty() {
            println!("\nNETWORK COMPARISON:");
            println!("{}", "-".repeat(40));
            self.print_network_comparison(&local_results, &remote_results);
        }

        println!("{}", "=".repeat(80));
    }

    /// Print summary for a specific network type
    fn print_network_summary(&self, results: &[&BenchmarkResult<T>]) {
        let mut table = Table::new();
        table.add_row(Row::new(vec![
            Cell::new("Benchmark"),
            Cell::new("Nodes"),
            Cell::new("Load (tx/s)"),
            Cell::new("Throughput (tx/s)"),
            Cell::new("Avg Latency (ms)"),
            Cell::new("Latency Std Dev (ms)"),
        ]));

        for result in results {
            if let Some(label) = result.measurements.labels().next() {
                let tps = result.measurements.aggregate_tps(label);
                let avg_latency = result.measurements.aggregate_average_latency(label);
                let stdev_latency = result.measurements.aggregate_stdev_latency(label);

                table.add_row(Row::new(vec![
                    Cell::new(&format!("{:?}", result.parameters.benchmark_type)),
                    Cell::new(&format!("{}", result.parameters.nodes)),
                    Cell::new(&format!("{}", result.parameters.load)),
                    Cell::new(&format!("{}", tps)),
                    Cell::new(&format!("{:.2}", avg_latency.as_millis())),
                    Cell::new(&format!("{:.2}", stdev_latency.as_millis())),
                ]));
            }
        }

        table.printstd();
    }

    /// Print comparison between local and remote networks
    fn print_network_comparison(
        &self,
        local_results: &[&BenchmarkResult<T>],
        remote_results: &[&BenchmarkResult<T>],
    ) {
        // Find comparable benchmarks (same parameters)
        for local_result in local_results {
            for remote_result in remote_results {
                if local_result.parameters.nodes == remote_result.parameters.nodes
                    && local_result.parameters.load == remote_result.parameters.load
                {
                    if let (Some(local_label), Some(remote_label)) = (
                        local_result.measurements.labels().next(),
                        remote_result.measurements.labels().next(),
                    ) {
                        let local_tps = local_result.measurements.aggregate_tps(local_label);
                        let remote_tps = remote_result.measurements.aggregate_tps(remote_label);
                        let local_latency = local_result
                            .measurements
                            .aggregate_average_latency(local_label);
                        let remote_latency = remote_result
                            .measurements
                            .aggregate_average_latency(remote_label);

                        let tps_diff = if local_tps > 0 {
                            ((remote_tps as f64 - local_tps as f64) / local_tps as f64) * 100.0
                        } else {
                            0.0
                        };

                        let latency_diff = if local_latency.as_millis() > 0 {
                            ((remote_latency.as_millis() as f64 - local_latency.as_millis() as f64)
                                / local_latency.as_millis() as f64)
                                * 100.0
                        } else {
                            0.0
                        };

                        println!(
                            "Comparison for {} nodes, {} tx/s load:",
                            local_result.parameters.nodes, local_result.parameters.load
                        );
                        println!(
                            "  Throughput: Local {} tx/s, Remote {} tx/s ({}%)",
                            local_tps, remote_tps, tps_diff
                        );
                        println!(
                            "  Latency: Local {:.2} ms, Remote {:.2} ms ({}%)",
                            local_latency.as_millis(),
                            remote_latency.as_millis(),
                            latency_diff
                        );
                        println!();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::{fmt::Display, str::FromStr, time::Duration};

    use serde::{Deserialize, Serialize};

    use crate::{
        faults::FaultsType,
        measurement::{Measurement, MeasurementsCollection},
        settings::Settings,
    };

    use super::{
        BenchmarkParameters, BenchmarkParametersGenerator, BenchmarkResult, BenchmarkRunner,
        BenchmarkType, LoadType, NetworkType,
    };

    /// Mock benchmark type for unit tests.
    #[derive(
        Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Default,
    )]
    pub struct TestBenchmarkType;

    impl Display for TestBenchmarkType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestBenchmarkType")
        }
    }

    impl FromStr for TestBenchmarkType {
        type Err = ();

        fn from_str(_s: &str) -> Result<Self, Self::Err> {
            Ok(Self {})
        }
    }

    impl BenchmarkType for TestBenchmarkType {}

    #[test]
    fn set_lower_bound() {
        let settings = Settings::new_for_test();
        let nodes = 4;
        let load = LoadType::Search {
            starting_load: 100,
            max_iterations: 10,
        };
        let mut generator = BenchmarkParametersGenerator::<TestBenchmarkType>::new(nodes, load);
        let parameters = generator.next().unwrap();

        let collection = MeasurementsCollection::new(&settings, parameters);
        generator.register_result(collection);

        let next_parameters = generator.next();
        assert!(next_parameters.is_some());
        assert_eq!(next_parameters.unwrap().load, 200);

        assert!(generator.lower_bound_result.is_some());
        assert_eq!(
            generator.lower_bound_result.unwrap().transaction_load(),
            100
        );
        assert!(generator.upper_bound_result.is_none());
    }

    #[test]
    fn set_upper_bound() {
        let settings = Settings::new_for_test();
        let nodes = 4;
        let load = LoadType::Search {
            starting_load: 100,
            max_iterations: 10,
        };
        let mut generator = BenchmarkParametersGenerator::<TestBenchmarkType>::new(nodes, load);
        let first_parameters = generator.next().unwrap();

        // Register a first result (zero latency). This sets the lower bound.
        let collection = MeasurementsCollection::new(&settings, first_parameters);
        generator.register_result(collection);
        let second_parameters = generator.next().unwrap();

        // Register a second result (with positive latency). This sets the upper bound.
        let mut collection = MeasurementsCollection::new(&settings, second_parameters);
        let (label, measurement) = Measurement::new_for_test();
        collection.add(1, label, measurement);
        generator.register_result(collection);

        // Ensure the next load is between the upper and the lower bound.
        let third_parameters = generator.next();
        assert!(third_parameters.is_some());
        assert_eq!(third_parameters.unwrap().load, 150);

        assert!(generator.lower_bound_result.is_some());
        assert_eq!(
            generator.lower_bound_result.unwrap().transaction_load(),
            100
        );
        assert!(generator.upper_bound_result.is_some());
        assert_eq!(
            generator.upper_bound_result.unwrap().transaction_load(),
            200
        );
    }

    #[test]
    fn max_iterations() {
        let settings = Settings::new_for_test();
        let nodes = 4;
        let load = LoadType::Search {
            starting_load: 100,
            max_iterations: 0,
        };
        let mut generator = BenchmarkParametersGenerator::<TestBenchmarkType>::new(nodes, load);
        let parameters = generator.next().unwrap();

        let collection = MeasurementsCollection::new(&settings, parameters);
        generator.register_result(collection);

        let next_parameters = generator.next();
        assert!(next_parameters.is_none());
    }

    #[test]
    fn benchmark_result_creation() {
        let settings = Settings::new_for_test();
        let parameters = BenchmarkParameters::new(
            TestBenchmarkType,
            4,
            FaultsType::Permanent { faults: 0 },
            100,
            Duration::from_secs(60),
        );
        let collection = MeasurementsCollection::new(&settings, parameters.clone());

        let result = BenchmarkResult::new(NetworkType::Local, parameters, collection);

        assert_eq!(result.network_type, NetworkType::Local);
        assert_eq!(result.parameters.nodes, 4);
        assert_eq!(result.parameters.load, 100);
        assert!(result.metadata.is_empty()); // Metadata starts empty
    }

    #[test]
    fn benchmark_runner_creation() {
        let output_dir = std::path::PathBuf::from("./test_results");
        let runner = BenchmarkRunner::<TestBenchmarkType>::new(output_dir.clone())
            .with_console_output(true)
            .with_file_output(true);

        assert_eq!(runner.output_dir, output_dir);
        assert!(runner.console_output);
        assert!(runner.file_output);
    }

    #[test]
    fn network_type_serialization() {
        let local = NetworkType::Local;
        let remote = NetworkType::Remote;

        let local_json = serde_json::to_string(&local).unwrap();
        let remote_json = serde_json::to_string(&remote).unwrap();

        assert_eq!(local_json, "\"Local\"");
        assert_eq!(remote_json, "\"Remote\"");

        let deserialized_local: NetworkType = serde_json::from_str(&local_json).unwrap();
        let deserialized_remote: NetworkType = serde_json::from_str(&remote_json).unwrap();

        assert_eq!(deserialized_local, NetworkType::Local);
        assert_eq!(deserialized_remote, NetworkType::Remote);
    }
}
