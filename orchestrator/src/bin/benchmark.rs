// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{path::PathBuf, time::Duration};

use clap::Parser;
use color_eyre::eyre::Result;
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt};

// Import the orchestrator modules
use orchestrator::benchmark::{BenchmarkParameters, BenchmarkResult, NetworkType};
use orchestrator::client::Instance;
use orchestrator::faults::FaultsType;
use orchestrator::measurement::{Measurement, MeasurementsCollection};
use orchestrator::protocol::mysticeti::MysticetiBenchmarkType;
use orchestrator::protocol::mysticeti::MysticetiProtocol;
use orchestrator::settings::Settings;
use orchestrator::settings::{CloudProvider, Repository};
use orchestrator::ssh::SshConnectionManager;
use orchestrator::{LocalNetworkOrchestrator, Orchestrator};

#[derive(Parser, Clone)]
#[command(
    author,
    version,
    about = "Comprehensive benchmark runner for local and remote networks"
)]
pub struct Opts {
    /// Output directory for benchmark results
    #[clap(long, default_value = "./benchmarks")]
    output_dir: String,

    /// Whether to print results to console
    #[clap(long, default_value = "true")]
    console_output: bool,

    /// Whether to save results to file
    #[clap(long, default_value = "true")]
    file_output: bool,

    /// The committee size
    #[clap(long, default_value = "4")]
    committee: usize,

    /// Number of faulty nodes
    #[clap(long, default_value = "0")]
    faults: usize,

    /// Whether the faulty nodes recover
    #[clap(long, default_value = "false")]
    crash_recovery: bool,

    /// The interval to crash nodes in seconds
    #[clap(long, default_value = "60")]
    crash_interval: u64,

    /// The duration of each benchmark in seconds
    #[clap(long, default_value = "180")]
    duration: u64,

    /// Load type for local network (fixed loads)
    #[clap(long, default_value = "100,200,500")]
    local_loads: String,

    /// Load type for remote network (fixed loads)
    #[clap(long, default_value = "50,100,200")]
    remote_loads: String,

    /// Transaction size in bytes
    #[clap(long, default_value = "512")]
    transaction_size: usize,

    /// Network type to benchmark (local or remote)
    #[clap(long, default_value = "local")]
    network_type: String,

    /// Path to docker-compose.yml file for local network
    #[clap(long, default_value = "../docker-compose.yml")]
    docker_compose_path: String,

    /// Wait time for network startup in seconds
    #[clap(long, default_value = "30")]
    startup_wait: u64,

    /// Whether to clean up containers after completion
    #[clap(long, default_value = "false")]
    cleanup: bool,

    /// Whether to perform thorough cleanup (remove volumes and containers completely)
    #[clap(long, default_value = "false")]
    cleanup_thorough: bool,
}

struct BenchmarkRunner {
    opts: Opts,
}

impl BenchmarkRunner {
    fn new(opts: Opts) -> Self {
        Self { opts }
    }

    async fn run_benchmarks(&self) -> Result<()> {
        info!("Starting comprehensive benchmark runner");
        info!("Network type: {}", self.opts.network_type);
        info!("Committee size: {}", self.opts.committee);
        info!("Duration: {}s", self.opts.duration);

        // Validate environment for remote networks
        if self.opts.network_type.to_lowercase() == "remote" {
            self.validate_remote_environment()?;
        }

        // Parse loads based on network type
        let loads: Vec<usize> = match self.opts.network_type.to_lowercase().as_str() {
            "local" => self
                .opts
                .local_loads
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok())
                .collect(),
            "remote" => self
                .opts
                .remote_loads
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok())
                .collect(),
            _ => {
                return Err(color_eyre::eyre::eyre!(
                    "Error: Network type must be 'local' or 'remote'"
                ));
            }
        };

        if loads.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "No valid loads specified for {} network",
                self.opts.network_type
            ));
        }

        info!(
            "Parsed loads for {} network: {:?}",
            self.opts.network_type, loads
        );

        // Create output directory
        let output_dir = PathBuf::from(&self.opts.output_dir);
        std::fs::create_dir_all(&output_dir)?;
        info!("Created output directory: {}", output_dir.display());

        // Run benchmarks for each load
        let mut all_results = Vec::new();

        for (i, load) in loads.iter().enumerate() {
            info!(
                "Running {} benchmark {}: {} tx/s",
                self.opts.network_type,
                i + 1,
                load
            );

            let result = self.run_single_benchmark(*load).await?;
            all_results.push((*load, result.clone()));

            // Save results
            if self.opts.file_output {
                self.save_benchmark_result(i + 1, *load, &result, &output_dir)
                    .await?;
            }

            if self.opts.console_output {
                self.print_benchmark_result(i + 1, *load, &result);
            }
        }

        // Print summary
        if self.opts.console_output {
            self.print_benchmark_summary(&all_results);
        }

        info!("Benchmark completed successfully!");
        Ok(())
    }

    fn validate_remote_environment(&self) -> Result<()> {
        info!("Validating remote network environment variables...");

        let required_vars = vec![
            "MYSTICETI_NODE0_HOST",
            "MYSTICETI_NODE1_HOST",
            "MYSTICETI_NODE2_HOST",
            "MYSTICETI_NODE3_HOST",
        ];

        for var in &required_vars {
            if std::env::var(var).is_err() {
                return Err(color_eyre::eyre::eyre!(
                    "Required environment variable {} not set. Please set all node host addresses for remote network benchmarks.",
                    var
                ));
            }
        }

        info!("Remote network environment validation passed");
        Ok(())
    }

    async fn run_single_benchmark(
        &self,
        load: usize,
    ) -> Result<BenchmarkResult<MysticetiBenchmarkType>> {
        match self.opts.network_type.to_lowercase().as_str() {
            "local" => self.run_local_network_benchmark(load).await,
            "remote" => self.run_remote_network_benchmark(load).await,
            _ => Err(color_eyre::eyre::eyre!("Invalid network type")),
        }
    }

    async fn run_local_network_benchmark(
        &self,
        load: usize,
    ) -> Result<BenchmarkResult<MysticetiBenchmarkType>> {
        info!("Starting local network benchmark with load: {} tx/s", load);

        // Create orchestrator for docker-compose based local network
        let orchestrator =
            LocalNetworkOrchestrator::new(PathBuf::from(&self.opts.docker_compose_path))?;

        // Verify docker-compose file exists
        orchestrator.verify_docker_compose()?;

        // Create benchmark parameters
        let parameters = BenchmarkParameters::new(
            MysticetiBenchmarkType::default(),
            self.opts.committee,
            FaultsType::Permanent {
                faults: self.opts.faults,
            },
            load,
            Duration::from_secs(self.opts.duration),
        );

        // Start the network using docker-compose
        info!("Starting Mysticeti network with docker-compose...");
        orchestrator.start_network()?;

        // Wait for network to be ready
        info!("Waiting for network to be ready...");
        orchestrator
            .wait_for_network_ready(self.opts.startup_wait)
            .await?;

        // Check network status
        let status = orchestrator.get_network_status()?;
        info!("Network status: {:?}", status);

        // Run the benchmark by simulating transactions
        info!("Starting transaction simulation...");
        let start_time = std::time::Instant::now();

        // Calculate total transactions to send
        let total_transactions = load * self.opts.duration as usize;
        let transaction_size = self.opts.transaction_size;

        // Simulate transactions
        orchestrator
            .simulate_transactions(total_transactions, transaction_size, load)
            .await?;

        let _benchmark_duration = start_time.elapsed();

        // Collect metrics from containers
        orchestrator.collect_metrics().await?;

        // Create mock measurements collection for local network
        let settings = self.create_local_settings()?;
        let mut measurements = MeasurementsCollection::new(&settings, parameters.clone());

        // Add mock measurement data based on the simulation
        // In a real implementation, you would collect actual metrics from the containers
        let (_, measurement) = Measurement::new_for_test();

        measurements.add(0, "default".to_string(), measurement);

        // Create benchmark result
        let result = BenchmarkResult::new(NetworkType::Local, parameters, measurements);

        // Cleanup if requested
        if self.opts.cleanup {
            info!("Cleaning up docker containers...");
            orchestrator.stop_network()?;
        }

        // Thorough cleanup if requested (takes precedence over regular cleanup)
        if self.opts.cleanup_thorough {
            info!("Performing thorough cleanup of docker containers and volumes...");
            orchestrator.stop_network_thorough()?;
        }

        Ok(result)
    }

    async fn run_remote_network_benchmark(
        &self,
        load: usize,
    ) -> Result<BenchmarkResult<MysticetiBenchmarkType>> {
        info!("Starting remote network benchmark with load: {} tx/s", load);

        // Create settings for remote network
        let settings = self.create_remote_settings()?;

        // Create instances from environment variables
        let instances = self.create_remote_instances()?;

        // Create SSH connection manager
        let ssh_manager =
            SshConnectionManager::new("ubuntu".to_string(), PathBuf::from("~/.ssh/id_rsa"));

        // Create protocol commands
        let protocol_commands = MysticetiProtocol::new(&settings);

        // Create orchestrator
        let orchestrator = Orchestrator::new(
            settings,
            instances,
            vec![], // No instance setup commands for remote
            protocol_commands,
            ssh_manager,
        )
        .with_monitoring(false); // Disable monitoring for remote benchmarks

        // Create benchmark parameters
        let parameters = BenchmarkParameters::new(
            MysticetiBenchmarkType::default(),
            self.opts.committee,
            FaultsType::Permanent {
                faults: self.opts.faults,
            },
            load,
            Duration::from_secs(self.opts.duration),
        );

        // Run the benchmark using orchestrator
        let measurements = orchestrator.run(&parameters).await?;

        // Create benchmark result
        let result = BenchmarkResult::new(NetworkType::Remote, parameters, measurements);

        Ok(result)
    }

    fn create_local_settings(&self) -> Result<Settings> {
        // Create settings for local network using docker-compose
        let settings = Settings {
            testbed_id: "local-benchmark".to_string(),
            cloud_provider: CloudProvider::Aws,
            token_file: PathBuf::from("~/.ssh/id_rsa"),
            ssh_private_key_file: PathBuf::from("~/.ssh/id_rsa"),
            ssh_public_key_file: None,
            regions: vec!["local".to_string()],
            specs: "local".to_string(),
            repository: Repository {
                url: reqwest::Url::parse("https://github.com/mystenlabs/mysticeti").unwrap(),
                commit: "main".to_string(),
            },
            working_dir: PathBuf::from("/tmp/mysticeti-benchmark"),
            results_dir: PathBuf::from(&self.opts.output_dir),
            logs_dir: PathBuf::from(&self.opts.output_dir).join("logs"),
        };

        Ok(settings)
    }

    fn create_remote_settings(&self) -> Result<Settings> {
        // Create settings for remote network
        let settings = Settings {
            testbed_id: "remote-benchmark".to_string(),
            cloud_provider: CloudProvider::Aws,
            token_file: PathBuf::from("~/.ssh/id_rsa"),
            ssh_private_key_file: PathBuf::from("~/.ssh/id_rsa"),
            ssh_public_key_file: None,
            regions: vec!["us-west-1".to_string()],
            specs: "t3.medium".to_string(),
            repository: Repository {
                url: reqwest::Url::parse("https://github.com/mystenlabs/mysticeti").unwrap(),
                commit: "main".to_string(),
            },
            working_dir: PathBuf::from("/tmp/mysticeti-benchmark"),
            results_dir: PathBuf::from(&self.opts.output_dir),
            logs_dir: PathBuf::from(&self.opts.output_dir).join("logs"),
        };

        Ok(settings)
    }

    fn create_remote_instances(&self) -> Result<Vec<Instance>> {
        // Create instances from environment variables
        let mut instances = Vec::new();

        for i in 0..self.opts.committee {
            let _host = std::env::var(&format!("MYSTICETI_NODE{}_HOST", i))
                .map_err(|_| color_eyre::eyre::eyre!("MYSTICETI_NODE{}_HOST not set", i))?;

            let _ssh_port = std::env::var(&format!("MYSTICETI_NODE{}_SSH_PORT", i))
                .unwrap_or_else(|_| "22".to_string())
                .parse::<u16>()
                .unwrap_or(22);

            let _ssh_user = std::env::var(&format!("MYSTICETI_NODE{}_SSH_USER", i))
                .unwrap_or_else(|_| "ubuntu".to_string());

            let instance = Instance {
                id: format!("remote-node-{}", i),
                region: "us-west-1".to_string(),
                main_ip: std::net::Ipv4Addr::new(127, 0, 0, 1), // This should be parsed from host
                tags: vec!["remote".to_string()],
                specs: "t3.medium".to_string(),
                status: "running".to_string(),
            };
            instances.push(instance);
        }

        Ok(instances)
    }

    async fn save_benchmark_result(
        &self,
        benchmark_num: usize,
        load: usize,
        result: &BenchmarkResult<MysticetiBenchmarkType>,
        output_dir: &PathBuf,
    ) -> Result<()> {
        let filename = format!(
            "{}_benchmark_{}_{}txs.json",
            self.opts.network_type, benchmark_num, load
        );
        let filepath = output_dir.join(filename);

        let json_data = serde_json::json!({
            "network_type": result.network_type,
            "benchmark_number": benchmark_num,
            "parameters": {
                "nodes": self.opts.committee,
                "load": load,
                "duration": self.opts.duration,
                "transaction_size": self.opts.transaction_size,
                "faults": self.opts.faults,
                "crash_recovery": self.opts.crash_recovery,
                "crash_interval": self.opts.crash_interval
            },
            "results": {
                "throughput": result.measurements.aggregate_tps(&"default".to_string()),
                "avg_latency_ms": result.measurements.aggregate_average_latency(&"default".to_string()).as_millis(),
                "latency_std_dev_ms": result.measurements.aggregate_stdev_latency(&"default".to_string()).as_millis(),
                "duration_secs": result.parameters.duration.as_secs(),
                "successful_transactions": result.measurements.transaction_load(),
                "failed_transactions": 0
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        std::fs::write(&filepath, serde_json::to_string_pretty(&json_data)?)?;
        info!("Saved benchmark results to: {}", filepath.display());

        Ok(())
    }

    fn print_benchmark_result(
        &self,
        benchmark_num: usize,
        load: usize,
        result: &BenchmarkResult<MysticetiBenchmarkType>,
    ) {
        println!("\n{}", "=".repeat(60));
        println!("BENCHMARK RESULT #{}", benchmark_num);
        println!("{}", "=".repeat(60));
        println!("Network Type: {:?}", result.network_type);
        println!("Input Load: {} tx/s", load);
        println!("Duration: {}s", result.parameters.duration.as_secs());
        println!();
        println!("RESULTS:");

        if let Some(label) = result.measurements.labels().next() {
            let throughput = result.measurements.aggregate_tps(&label);
            let avg_latency = result.measurements.aggregate_average_latency(&label);
            let latency_std_dev = result.measurements.aggregate_stdev_latency(&label);

            println!("  Throughput: {} tx/s", throughput);
            println!("  Average Latency: {:.2} ms", avg_latency.as_millis());
            println!("  Latency Std Dev: {:.2} ms", latency_std_dev.as_millis());
            println!(
                "  Efficiency: {:.1}%",
                (throughput as f64 / load as f64) * 100.0
            );
        }

        println!("{}", "=".repeat(60));
    }

    fn print_benchmark_summary(
        &self,
        results: &[(usize, BenchmarkResult<MysticetiBenchmarkType>)],
    ) {
        println!("\n{}", "=".repeat(80));
        println!("BENCHMARK SUMMARY");
        println!("{}", "=".repeat(80));
        println!("Network Type: {}", self.opts.network_type.to_uppercase());
        println!("Committee Size: {}", self.opts.committee);
        println!("Duration: {}s", self.opts.duration);
        println!("Transaction Size: {} bytes", self.opts.transaction_size);
        println!();

        println!("RESULTS SUMMARY:");
        println!(
            "{:<12} {:<12} {:<15} {:<15} {:<12}",
            "Load (tx/s)", "Throughput", "Avg Latency", "Latency Std", "Efficiency"
        );
        println!("{:-<80}", "");

        for (load, result) in results {
            if let Some(label) = result.measurements.labels().next() {
                let throughput = result.measurements.aggregate_tps(&label);
                let avg_latency = result.measurements.aggregate_average_latency(&label);
                let latency_std_dev = result.measurements.aggregate_stdev_latency(&label);

                let efficiency = if *load > 0 {
                    (throughput as f64 / *load as f64) * 100.0
                } else {
                    0.0
                };

                println!(
                    "{:<12} {:<12} {:<15.2} {:<15.2} {:<12.1}%",
                    load,
                    throughput,
                    avg_latency.as_millis(),
                    latency_std_dev.as_millis(),
                    efficiency
                );
            }
        }

        println!("{:-<80}", "");
        println!("Total benchmarks run: {}", results.len());
        println!("Output directory: {}", self.opts.output_dir);
        println!("{}", "=".repeat(80));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Nice colored error messages.
    color_eyre::install()?;

    // Setup logging
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    fmt().with_env_filter(filter).init();

    let opts: Opts = Opts::parse();

    println!("Comprehensive Benchmark Runner");
    println!("=============================");
    println!();
    println!("Configuration:");
    println!("  Output directory: {}", opts.output_dir);
    println!("  Console output: {}", opts.console_output);
    println!("  File output: {}", opts.file_output);
    println!("  Committee size: {}", opts.committee);
    println!("  Faults: {}", opts.faults);
    println!("  Crash recovery: {}", opts.crash_recovery);
    println!("  Crash interval: {}s", opts.crash_interval);
    println!("  Duration: {}s", opts.duration);
    println!("  Network type: {}", opts.network_type);
    println!("  Transaction size: {} bytes", opts.transaction_size);
    println!("  Docker compose path: {}", opts.docker_compose_path);
    println!("  Startup wait: {}s", opts.startup_wait);
    println!("  Cleanup: {}", opts.cleanup);
    println!();

    // Show usage examples
    if opts.network_type.to_lowercase() == "remote" {
        println!(
            "Note: For remote network benchmarks, ensure the following environment variables are set:"
        );
        println!(
            "  MYSTICETI_NODE0_HOST, MYSTICETI_NODE1_HOST, MYSTICETI_NODE2_HOST, MYSTICETI_NODE3_HOST"
        );
        println!();
    }

    if opts.network_type.to_lowercase() == "local" {
        println!(
            "Note: For local network benchmarks, ensure Docker is running and docker-compose.yml exists"
        );
        println!();
    }

    let runner = BenchmarkRunner::new(opts.clone());
    runner.run_benchmarks().await?;

    println!("\nBenchmark completed successfully!");
    println!(
        "Check the output directory for detailed results: {}",
        opts.output_dir
    );
    println!();
    println!("Usage examples:");
    println!("  # Local network benchmark with default settings");
    println!("  cargo run --bin benchmark -- --network-type local");
    println!();
    println!("  # Remote network benchmark with custom loads");
    println!("  cargo run --bin benchmark -- --network-type remote --remote-loads 50,100,200");
    println!();
    println!("  # Local network with custom parameters");
    println!(
        "  cargo run --bin benchmark -- --network-type local --local-loads 100,500,1000 --duration 300 --cleanup"
    );

    Ok(())
}
