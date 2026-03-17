use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use tracing::{error, info};

use ratiomaster_core::client::profiles;
use ratiomaster_core::config::{self, AppConfig};
use ratiomaster_core::engine::speed::SpeedConfig;
use ratiomaster_core::engine::stop::StopCondition;
use ratiomaster_core::engine::{Engine, EngineConfig};
use ratiomaster_core::proxy::ProxyConfig;
use ratiomaster_core::torrent;

/// RatioMaster-Rust: BitTorrent tracker communication tool.
#[derive(Parser)]
#[command(name = "ratiomaster-cli", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to .torrent file.
    #[arg(value_name = "TORRENT_FILE")]
    torrent_file: Option<PathBuf>,

    /// Client to emulate.
    #[arg(short, long, default_value = "uTorrent 3.3.2")]
    client: String,

    /// Upload speed in KB/s.
    #[arg(short = 'u', long = "upload", default_value_t = 100)]
    upload_speed: u64,

    /// Download speed in KB/s.
    #[arg(short = 'd', long = "download", default_value_t = 0)]
    download_speed: u64,

    /// Override tracker URL.
    #[arg(short = 't', long)]
    tracker: Option<String>,

    /// Override listening port.
    #[arg(short = 'p', long)]
    port: Option<u16>,

    /// Override announce interval in seconds.
    #[arg(short = 'i', long)]
    interval: Option<u64>,

    /// Bytes already downloaded.
    #[arg(short = 's', long)]
    downloaded: Option<u64>,

    /// Random upload range in KB/s (MIN:MAX).
    #[arg(long, value_name = "MIN:MAX")]
    upload_random: Option<String>,

    /// Random download range in KB/s (MIN:MAX).
    #[arg(long, value_name = "MIN:MAX")]
    download_random: Option<String>,

    /// Stop after uploading N bytes.
    #[arg(long)]
    stop_upload: Option<u64>,

    /// Stop after downloading N bytes.
    #[arg(long)]
    stop_download: Option<u64>,

    /// Stop after N seconds.
    #[arg(long)]
    stop_time: Option<u64>,

    /// Stop after reaching ratio.
    #[arg(long)]
    stop_ratio: Option<f64>,

    /// Proxy type: socks4, socks4a, socks5, http.
    #[arg(long)]
    proxy_type: Option<String>,

    /// Proxy host.
    #[arg(long)]
    proxy_host: Option<String>,

    /// Proxy port.
    #[arg(long)]
    proxy_port: Option<u16>,

    /// Proxy username.
    #[arg(long)]
    proxy_user: Option<String>,

    /// Proxy password.
    #[arg(long)]
    proxy_pass: Option<String>,

    /// Enable TCP handshake listener.
    #[arg(long)]
    tcp_listener: bool,

    /// Enable scrape requests.
    #[arg(long)]
    scrape: bool,

    /// Ignore tracker failure reasons.
    #[arg(long)]
    ignore_failure: bool,

    /// Custom peer ID.
    #[arg(long)]
    custom_peer_id: Option<String>,

    /// Custom tracker key.
    #[arg(long)]
    custom_key: Option<String>,

    /// Load config from TOML file.
    #[arg(long)]
    config: Option<PathBuf>,

    /// Write logs to file.
    #[arg(long)]
    log_file: Option<PathBuf>,

    /// Suppress output (only errors).
    #[arg(long)]
    quiet: bool,

    /// Verbose logging.
    #[arg(long)]
    verbose: bool,

    /// List all available client profiles.
    #[arg(long)]
    list_clients: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run multiple torrents in batch mode.
    Batch {
        /// Directories or .torrent files to process.
        #[arg(required = true)]
        paths: Vec<PathBuf>,

        /// Shared config file for all torrents.
        #[arg(long)]
        config: Option<PathBuf>,
    },

    /// Store proxy credentials in the system keyring.
    #[cfg(feature = "keyring")]
    StoreCredential {
        /// Proxy username.
        #[arg(long)]
        username: String,

        /// Proxy password.
        #[arg(long)]
        password: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Handle --list-clients
    if cli.list_clients {
        print_client_list();
        return;
    }

    // Set up logging
    setup_logging(&cli);

    // Handle subcommands
    match cli.command {
        Some(Commands::Batch { paths, config }) => {
            run_batch(paths, config).await;
            return;
        }
        #[cfg(feature = "keyring")]
        Some(Commands::StoreCredential { username, password }) => {
            match ratiomaster_core::config::credential_store::store_password(&username, &password) {
                Ok(()) => println!("Credential stored for '{username}'"),
                Err(e) => {
                    eprintln!("error: failed to store credential: {e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        None => {}
    }

    // Single torrent mode requires a torrent file
    let torrent_path = match cli.torrent_file {
        Some(ref path) => path.clone(),
        None => {
            eprintln!("error: a torrent file is required (or use --list-clients)");
            eprintln!("usage: ratiomaster-cli [OPTIONS] <TORRENT_FILE>");
            std::process::exit(1);
        }
    };

    run_single(cli, torrent_path).await;
}

fn setup_logging(cli: &Cli) {
    use tracing_subscriber::EnvFilter;

    let filter = if cli.quiet {
        "error"
    } else if cli.verbose {
        "debug"
    } else {
        "info"
    };

    let builder = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(false);

    if let Some(ref log_file) = cli.log_file {
        let file = std::fs::File::create(log_file).expect("failed to create log file");
        builder
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false)
            .init();
    } else {
        builder.init();
    }
}

fn print_client_list() {
    println!("Available client profiles:");
    println!("{:<4} Name", "#");
    println!("{}", "-".repeat(40));
    for (i, profile) in profiles::all_profiles().iter().enumerate() {
        println!("{:<4} {}", i + 1, profile.name);
    }
    println!("\n{} profiles available.", profiles::all_profiles().len());
}

async fn run_single(cli: Cli, torrent_path: PathBuf) {
    // Load base config
    let base_config = if let Some(ref config_path) = cli.config {
        match config::load_config_from(config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "error: failed to load config {}: {e}",
                    config_path.display()
                );
                std::process::exit(1);
            }
        }
    } else {
        config::load_config()
    };

    // Parse torrent file
    let torrent_data = match std::fs::read(&torrent_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("error: failed to read {}: {e}", torrent_path.display());
            std::process::exit(1);
        }
    };

    let mut torrent_meta = match torrent::parse(&torrent_data) {
        Ok(meta) => meta,
        Err(e) => {
            eprintln!("error: failed to parse torrent: {e}");
            std::process::exit(1);
        }
    };

    // Override tracker URL if specified
    if let Some(ref tracker) = cli.tracker {
        torrent_meta.announce = tracker.clone();
    }

    // Look up client profile
    let profile = match profiles::get_profile(&cli.client) {
        Some(p) => p.clone(),
        None => {
            eprintln!(
                "error: unknown client profile '{}'. Use --list-clients to see options.",
                cli.client
            );
            std::process::exit(1);
        }
    };

    // Build engine config from base + CLI overrides
    let engine_config = build_engine_config(&cli, &base_config);

    // Build proxy config
    let proxy = build_proxy_config(&cli, &base_config);

    info!("torrent: {}", torrent_meta.name);
    info!("tracker: {}", torrent_meta.announce);
    info!(
        "size: {} ({} bytes)",
        format_bytes(torrent_meta.total_size()),
        torrent_meta.total_size()
    );
    info!("client: {}", profile.name);
    info!(
        "info hash: {}",
        torrent_meta
            .info_hash
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );

    // Create and run engine
    let mut engine = Engine::new(torrent_meta, profile, proxy, engine_config);
    let state_rx = engine.subscribe_state();
    let shutdown = engine.shutdown_handle();

    // Handle Ctrl+C
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("shutting down...");
        let _ = shutdown_clone.send(true);
    });

    // Status line printer
    tokio::spawn(print_status_loop(state_rx));

    // Run engine
    if let Err(e) = engine.run().await {
        error!("engine error: {e}");
        std::process::exit(1);
    }

    info!("session complete");

    // Save session
    let state = engine.state();
    let session = ratiomaster_core::config::session::Session {
        torrent_path: torrent_path.to_string_lossy().into_owned(),
        uploaded: state.uploaded,
        downloaded: state.downloaded,
        left: state.left,
        client_name: cli.client.clone(),
        port: engine_config_port(&cli, &base_config),
        upload_speed: cli.upload_speed,
        download_speed: cli.download_speed,
        interval: state.interval,
        tcp_listener: cli.tcp_listener,
        scrape_enabled: cli.scrape,
        tracker_override: cli.tracker.clone(),
    };
    if let Err(e) = ratiomaster_core::config::session::save_session(&session) {
        error!("failed to save session: {e}");
    }
}

async fn run_batch(paths: Vec<PathBuf>, config_path: Option<PathBuf>) {
    let base_config = if let Some(ref path) = config_path {
        match config::load_config_from(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: failed to load config {}: {e}", path.display());
                std::process::exit(1);
            }
        }
    } else {
        config::load_config()
    };

    // Collect all .torrent files
    let mut torrent_files = Vec::new();
    for path in &paths {
        if path.is_dir() {
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.extension().is_some_and(|ext| ext == "torrent") {
                            torrent_files.push(p);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("warning: failed to read directory {}: {e}", path.display());
                }
            }
        } else {
            torrent_files.push(path.clone());
        }
    }

    if torrent_files.is_empty() {
        eprintln!("error: no .torrent files found");
        std::process::exit(1);
    }

    info!("batch mode: {} torrent files", torrent_files.len());

    let profile = profiles::get_profile(&base_config.general.default_client)
        .unwrap_or(&profiles::all_profiles()[0])
        .clone();

    let proxy = base_config.proxy.to_proxy_config();
    let engine_config = base_config.to_engine_config();

    let mut batch = ratiomaster_core::engine::batch::BatchEngine::new();

    for path in &torrent_files {
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("warning: failed to read {}: {e}", path.display());
                continue;
            }
        };

        match torrent::parse(&data) {
            Ok(meta) => {
                info!(
                    "  adding: {} ({})",
                    meta.name,
                    format_bytes(meta.total_size())
                );
                batch.add(meta, profile.clone(), proxy.clone(), engine_config.clone());
            }
            Err(e) => {
                eprintln!("warning: failed to parse {}: {e}", path.display());
            }
        }
    }

    if batch.is_empty() {
        eprintln!("error: no valid torrents to process");
        std::process::exit(1);
    }

    let count = batch.len();
    batch.start_all();
    info!("started {count} engines");

    // Handle Ctrl+C by stopping all
    // We need to stop_all from the signal handler, but batch is not Send-safe
    // for spawning. Instead, rely on process exit cleaning up the engines.
    tokio::spawn(async {
        tokio::signal::ctrl_c().await.ok();
        info!("shutting down batch (press Ctrl+C again to force)...");
        std::process::exit(0);
    });

    let results = batch.join_all().await;

    let mut success = 0;
    let mut failed = 0;
    for result in &results {
        match result {
            Ok(()) => success += 1,
            Err(e) => {
                error!("engine error: {e}");
                failed += 1;
            }
        }
    }

    info!("batch complete: {success} succeeded, {failed} failed");
}

fn build_engine_config(cli: &Cli, base: &AppConfig) -> EngineConfig {
    let port = engine_config_port(cli, base);

    // Speed config
    let (upload_min, upload_max) = if let Some(ref range) = cli.upload_random {
        parse_range(range, cli.upload_speed)
    } else if base.upload.random_enabled {
        (base.upload.random_min * 1024, base.upload.random_max * 1024)
    } else {
        (cli.upload_speed * 1024, cli.upload_speed * 1024)
    };

    let (download_min, download_max) = if let Some(ref range) = cli.download_random {
        parse_range(range, cli.download_speed)
    } else if base.download.random_enabled {
        (
            base.download.random_min * 1024,
            base.download.random_max * 1024,
        )
    } else {
        (cli.download_speed * 1024, cli.download_speed * 1024)
    };

    // Stop condition (CLI overrides config)
    let stop = if let Some(bytes) = cli.stop_upload {
        StopCondition::AfterUpload(bytes)
    } else if let Some(bytes) = cli.stop_download {
        StopCondition::AfterDownload(bytes)
    } else if let Some(secs) = cli.stop_time {
        StopCondition::AfterTime(Duration::from_secs(secs))
    } else if let Some(ratio) = cli.stop_ratio {
        StopCondition::AfterRatio(ratio)
    } else {
        base.stop.to_stop_condition()
    };

    // Calculate initial downloaded percentage
    let initial_downloaded_percent = if cli.downloaded.is_some() {
        // Will be handled separately as raw bytes, set to 0 here
        0
    } else {
        0
    };

    EngineConfig {
        port,
        speed: SpeedConfig {
            upload_min,
            upload_max,
            download_min,
            download_max,
            variation: 10 * 1024,
        },
        stop_condition: stop,
        ignore_failure: cli.ignore_failure || base.general.ignore_failure_reason,
        max_retries: 5,
        initial_downloaded_percent,
        http_timeout: Duration::from_secs(30),
        bind_address: "127.0.0.1".into(),
    }
}

fn engine_config_port(cli: &Cli, _base: &AppConfig) -> u16 {
    cli.port.unwrap_or(6881)
}

fn build_proxy_config(cli: &Cli, base: &AppConfig) -> ProxyConfig {
    if let Some(ref proxy_type) = cli.proxy_type {
        let host = cli
            .proxy_host
            .clone()
            .unwrap_or_else(|| base.proxy.host.clone());
        let port = cli.proxy_port.unwrap_or(base.proxy.port);
        let user = cli
            .proxy_user
            .clone()
            .unwrap_or_else(|| base.proxy.username.clone());
        let pass = cli
            .proxy_pass
            .clone()
            .unwrap_or_else(|| base.proxy.password.clone());

        match proxy_type.as_str() {
            "socks4" => ProxyConfig::Socks4 {
                proxy_host: host,
                proxy_port: port,
                user_id: user,
            },
            "socks4a" => ProxyConfig::Socks4a {
                proxy_host: host,
                proxy_port: port,
                user_id: user,
            },
            "socks5" => ProxyConfig::Socks5 {
                proxy_host: host,
                proxy_port: port,
                credentials: if user.is_empty() {
                    None
                } else {
                    Some(ratiomaster_core::proxy::socks5::Credentials {
                        username: user,
                        password: pass,
                    })
                },
            },
            "http" => ProxyConfig::HttpConnect {
                proxy_host: host,
                proxy_port: port,
                credentials: if user.is_empty() {
                    None
                } else {
                    Some(ratiomaster_core::proxy::http::Credentials {
                        username: user,
                        password: pass,
                    })
                },
            },
            other => {
                eprintln!(
                    "error: unknown proxy type '{other}' (use socks4, socks4a, socks5, http)"
                );
                std::process::exit(1);
            }
        }
    } else {
        base.proxy.to_proxy_config()
    }
}

/// Parses a range string like "50:150" into (min*1024, max*1024).
fn parse_range(s: &str, default: u64) -> (u64, u64) {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let min = parts[0].parse::<u64>().unwrap_or(default) * 1024;
        let max = parts[1].parse::<u64>().unwrap_or(default) * 1024;
        (min, max)
    } else {
        (default * 1024, default * 1024)
    }
}

/// Prints a real-time status line from engine state updates.
async fn print_status_loop(
    mut state_rx: tokio::sync::watch::Receiver<ratiomaster_core::engine::EngineState>,
) {
    loop {
        if state_rx.changed().await.is_err() {
            break;
        }

        let state = state_rx.borrow().clone();
        let elapsed = state.started_at.elapsed();
        let hours = elapsed.as_secs() / 3600;
        let minutes = (elapsed.as_secs() % 3600) / 60;
        let seconds = elapsed.as_secs() % 60;

        eprint!(
            "\r[{hours:02}:{minutes:02}:{seconds:02}] Announce #{} | Up: {} | Down: {} | Seeders: {} | Leechers: {} | Next: {}s   ",
            state.announce_count,
            format_bytes(state.uploaded),
            format_bytes(state.downloaded),
            state.seeders,
            state.leechers,
            state.interval,
        );
    }
    eprintln!();
}

/// Formats a byte count into a human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_values() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
        assert_eq!(format_bytes(1_099_511_627_776), "1.00 TB");
    }

    #[test]
    fn parse_range_valid() {
        assert_eq!(parse_range("50:150", 100), (50 * 1024, 150 * 1024));
    }

    #[test]
    fn parse_range_invalid() {
        assert_eq!(parse_range("bad", 100), (100 * 1024, 100 * 1024));
    }

    #[test]
    fn parse_range_partial() {
        assert_eq!(parse_range("50:bad", 100), (50 * 1024, 100 * 1024));
    }
}
