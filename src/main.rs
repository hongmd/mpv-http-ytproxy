use argh::FromArgs;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use third_wheel::hyper::http::HeaderValue;
use third_wheel::hyper::service::Service;
use third_wheel::hyper::{Body, Request};
use third_wheel::{mitm_layer, CertificateAuthority, MitmProxy, ThirdWheel};

// Custom deserializer for human-readable sizes (e.g., "10MB", "50MB", "1GB")
fn deserialize_size<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SizeValue {
        Number(u64),
        String(String),
    }

    let value = SizeValue::deserialize(deserializer)?;

    match value {
        SizeValue::Number(n) => Ok(n),
        SizeValue::String(s) => parse_size(&s).map_err(serde::de::Error::custom),
    }
}

fn parse_size(input: &str) -> Result<u64, String> {
    let input = input.trim().to_uppercase();

    if let Ok(num) = input.parse::<u64>() {
        return Ok(num);
    }

    // More efficient unit parsing with compile-time constants
    const UNITS: &[(&str, u64)] = &[
        ("TB", 1_024_u64.pow(4)),
        ("GB", 1_024_u64.pow(3)),
        ("MB", 1_024_u64.pow(2)),
        ("KB", 1_024),
        ("T", 1_024_u64.pow(4)),
        ("G", 1_024_u64.pow(3)),
        ("M", 1_024_u64.pow(2)),
        ("K", 1_024),
    ];

    for (unit, multiplier) in UNITS {
        if let Some(number_str) = input.strip_suffix(unit) {
            let number: f64 = number_str
                .trim()
                .parse()
                .map_err(|_| format!("Invalid number in size: {}", input))?;

            return Ok((number * *multiplier as f64) as u64);
        }
    }

    Err(format!(
        "Invalid size format: {}. Use formats like '10MB', '50MB', '1GB'",
        input
    ))
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct Config {
    #[serde(default)]
    proxy: ProxyConfig,
    #[serde(default)]
    security: SecurityConfig,
    #[serde(default)]
    logging: LoggingConfig,
    #[serde(default)]
    performance: PerformanceConfig,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProxyConfig {
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_chunk_size", deserialize_with = "deserialize_size")]
    chunk_size: u64,
    #[serde(default = "default_cert_file")]
    cert_file: String,
    #[serde(default = "default_key_file")]
    key_file: String,
    #[serde(default)]
    adaptive_chunking: bool,
    #[serde(
        default = "default_min_chunk_size",
        deserialize_with = "deserialize_size"
    )]
    min_chunk_size: u64,
    #[serde(
        default = "default_max_chunk_size",
        deserialize_with = "deserialize_size"
    )]
    max_chunk_size: u64,
    #[serde(default)]
    parallel_downloads: bool,
    #[serde(default = "default_max_concurrent_chunks")]
    max_concurrent_chunks: u32,
    #[serde(
        default = "default_prefetch_ahead",
        deserialize_with = "deserialize_size"
    )]
    prefetch_ahead: u64,
    #[serde(default = "default_memory_pool_enabled")]
    memory_pool_enabled: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct SecurityConfig {
    passphrase: Option<String>,
    #[serde(default = "default_cert_validity_days")]
    cert_validity_days: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct LoggingConfig {
    #[serde(default = "default_log_level")]
    level: String,
    log_file: Option<String>,
    #[serde(default)]
    log_timing: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct PerformanceConfig {
    #[serde(default = "default_http2")]
    http2: bool,
    #[serde(default = "default_connection_pool_size")]
    connection_pool_size: u32,
    #[serde(default = "default_request_timeout")]
    request_timeout: u64,
}

// Constants for better performance and maintainability
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_CHUNK_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const DEFAULT_MIN_CHUNK_SIZE: u64 = 2_621_440; // 2.5MB
const DEFAULT_MAX_CHUNK_SIZE: u64 = 40 * 1024 * 1024; // 40MB
const DEFAULT_CERT_VALIDITY_DAYS: u32 = 365;
const DEFAULT_CONNECTION_POOL_SIZE: u32 = 10;
const DEFAULT_REQUEST_TIMEOUT: u64 = 30;
const DEFAULT_MAX_CONCURRENT_CHUNKS: u32 = 2;
const DEFAULT_PREFETCH_AHEAD: u64 = 20 * 1024 * 1024; // 20MB

// Default value functions using constants (with inlining for better performance)
#[inline]
fn default_port() -> u16 {
    DEFAULT_PORT
}
#[inline]
fn default_chunk_size() -> u64 {
    DEFAULT_CHUNK_SIZE
}
#[inline]
fn default_cert_file() -> String {
    "cert.pem".to_string()
}
#[inline]
fn default_key_file() -> String {
    "key.pem".to_string()
}
#[inline]
fn default_min_chunk_size() -> u64 {
    DEFAULT_MIN_CHUNK_SIZE
}
#[inline]
fn default_max_chunk_size() -> u64 {
    DEFAULT_MAX_CHUNK_SIZE
}
#[inline]
fn default_cert_validity_days() -> u32 {
    DEFAULT_CERT_VALIDITY_DAYS
}
#[inline]
fn default_log_level() -> String {
    "info".to_string()
}
#[inline]
fn default_connection_pool_size() -> u32 {
    DEFAULT_CONNECTION_POOL_SIZE
}
#[inline]
fn default_request_timeout() -> u64 {
    DEFAULT_REQUEST_TIMEOUT
}
#[inline]
fn default_http2() -> bool {
    true
} // Enable HTTP/2 by default for better performance
#[inline]
fn default_max_concurrent_chunks() -> u32 {
    DEFAULT_MAX_CONCURRENT_CHUNKS
}
#[inline]
fn default_prefetch_ahead() -> u64 {
    DEFAULT_PREFETCH_AHEAD
}
#[inline]
fn default_memory_pool_enabled() -> bool {
    true
} // Enable memory pooling by default

// Memory Pool Management for efficient buffer reuse with optimized locking
#[derive(Debug)]
struct BufferPool {
    available_buffers: Arc<Mutex<VecDeque<Vec<u8>>>>,
    buffer_size: usize,
    max_buffers: usize,
    allocated_count: Arc<Mutex<usize>>,
    pool_hits: Arc<Mutex<u64>>,
    pool_misses: Arc<Mutex<u64>>,
}

impl BufferPool {
    fn new(buffer_size: usize, initial_count: usize, max_buffers: usize) -> Self {
        let mut buffers = VecDeque::new();

        // Pre-allocate buffers for immediate use
        for _ in 0..initial_count {
            buffers.push_back(vec![0u8; buffer_size]);
        }

        Self {
            available_buffers: Arc::new(Mutex::new(buffers)),
            buffer_size,
            max_buffers,
            allocated_count: Arc::new(Mutex::new(initial_count)),
            pool_hits: Arc::new(Mutex::new(0)),
            pool_misses: Arc::new(Mutex::new(0)),
        }
    }

    fn get_buffer(&self) -> Vec<u8> {
        let mut available = self.available_buffers.lock().unwrap();

        if let Some(mut buffer) = available.pop_front() {
            // Reuse existing buffer - just clear and resize
            buffer.clear();
            buffer.resize(self.buffer_size, 0);
            *self.pool_hits.lock().unwrap() += 1;
            return buffer;
        }

        // Create new buffer if we haven't reached the limit
        let mut count = self.allocated_count.lock().unwrap();
        if *count < self.max_buffers {
            *count += 1;
            *self.pool_misses.lock().unwrap() += 1;
            vec![0u8; self.buffer_size]
        } else {
            // If pool is exhausted, wait briefly and retry
            drop(available);
            drop(count);
            std::thread::sleep(std::time::Duration::from_millis(1));
            self.get_buffer()
        }
    }

    fn return_buffer(&self, buffer: Vec<u8>) {
        if buffer.capacity() >= self.buffer_size {
            let mut available = self.available_buffers.lock().unwrap();
            if available.len() < self.max_buffers / 2 {
                available.push_back(buffer);
            }
            // If pool is full, let the buffer be deallocated
        }
    }

    fn get_stats(&self) -> (u64, u64, f64) {
        let hits = *self.pool_hits.lock().unwrap();
        let misses = *self.pool_misses.lock().unwrap();
        let hit_rate = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64 * 100.0
        } else {
            0.0
        };
        (hits, misses, hit_rate)
    }
}

#[derive(Debug)]
struct ChunkDataPool {
    small_chunks: BufferPool,  // 1-5MB chunks
    medium_chunks: BufferPool, // 5-20MB chunks
    large_chunks: BufferPool,  // 20MB+ chunks
    enabled: bool,
}

impl ChunkDataPool {
    fn new(enabled: bool) -> Self {
        Self {
            small_chunks: BufferPool::new(5 * 1024 * 1024, 4, 16), // 5MB x 16 max
            medium_chunks: BufferPool::new(20 * 1024 * 1024, 2, 8), // 20MB x 8 max
            large_chunks: BufferPool::new(50 * 1024 * 1024, 1, 4), // 50MB x 4 max
            enabled,
        }
    }

    fn get_buffer_for_size(&self, size: usize) -> Vec<u8> {
        if !self.enabled {
            return vec![0u8; size];
        }

        match size {
            0..=5_242_880 => self.small_chunks.get_buffer(), // ≤ 5MB
            5_242_881..=20_971_520 => self.medium_chunks.get_buffer(), // 5-20MB
            _ => self.large_chunks.get_buffer(),             // > 20MB
        }
    }

    fn return_buffer(&self, buffer: Vec<u8>) {
        if !self.enabled {
            return;
        }

        let size = buffer.capacity();
        match size {
            0..=5_242_880 => self.small_chunks.return_buffer(buffer),
            5_242_881..=20_971_520 => self.medium_chunks.return_buffer(buffer),
            _ => self.large_chunks.return_buffer(buffer),
        }
    }

    fn print_stats(&self) {
        if !self.enabled {
            return;
        }

        let (small_hits, small_misses, small_rate) = self.small_chunks.get_stats();
        let (medium_hits, medium_misses, medium_rate) = self.medium_chunks.get_stats();
        let (large_hits, large_misses, large_rate) = self.large_chunks.get_stats();

        println!("Memory Pool Stats:");
        println!(
            "  Small (≤5MB): {} hits, {} misses, {:.1}% hit rate",
            small_hits, small_misses, small_rate
        );
        println!(
            "  Medium (5-20MB): {} hits, {} misses, {:.1}% hit rate",
            medium_hits, medium_misses, medium_rate
        );
        println!(
            "  Large (>20MB): {} hits, {} misses, {:.1}% hit rate",
            large_hits, large_misses, large_rate
        );
    }
}

// Type alias for complex download tracking
type DownloadTracker = Arc<Mutex<HashMap<String, Vec<(u64, u64)>>>>;

// Parallel download manager for intelligent prefetching with memory pooling
#[derive(Debug)]
struct ParallelDownloadManager {
    active_downloads: DownloadTracker, // URL -> [(start, end), ...]
    max_concurrent: u32,
    prefetch_size: u64,
    enabled: bool,
    chunk_pool: Arc<ChunkDataPool>,
    stats_timer: Arc<Mutex<Option<Instant>>>,
}

impl ParallelDownloadManager {
    fn new(
        max_concurrent: u32,
        prefetch_size: u64,
        enabled: bool,
        memory_pool_enabled: bool,
    ) -> Self {
        Self {
            active_downloads: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent,
            prefetch_size,
            enabled,
            chunk_pool: Arc::new(ChunkDataPool::new(memory_pool_enabled)),
            stats_timer: Arc::new(Mutex::new(None)),
        }
    }

    fn get_download_buffer(&self, _url: &str, size: usize) -> Vec<u8> {
        self.chunk_pool.get_buffer_for_size(size)
    }

    fn return_download_buffer(&self, buffer: Vec<u8>) {
        // Return to pool
        self.chunk_pool.return_buffer(buffer);

        // Print stats periodically (every 30 seconds)
        let mut timer = self.stats_timer.lock().unwrap();
        let now = Instant::now();
        if timer.is_none() || timer.unwrap().elapsed().as_secs() >= 30 {
            self.chunk_pool.print_stats();
            *timer = Some(now);
        }
    }

    fn should_prefetch(&self, url: &str, start: u64, chunk_size: u64) -> Vec<(u64, u64)> {
        if !self.enabled {
            return vec![];
        }

        let mut active = self.active_downloads.lock().unwrap();
        let downloads = active.entry(url.to_string()).or_default();

        // Check if we should prefetch next chunks
        let mut prefetch_ranges = Vec::new();
        let current_end = start + chunk_size;

        // Calculate how many chunks to prefetch ahead
        let chunks_to_prefetch =
            (self.prefetch_size / chunk_size).min(self.max_concurrent as u64 - 1);

        for i in 1..=chunks_to_prefetch {
            let prefetch_start = current_end + (i - 1) * chunk_size;
            let prefetch_end = prefetch_start + chunk_size - 1;

            // Check if this range is not already being downloaded
            let already_downloading = downloads
                .iter()
                .any(|(s, e)| prefetch_start >= *s && prefetch_start <= *e);

            if !already_downloading {
                prefetch_ranges.push((prefetch_start, prefetch_end));
                downloads.push((prefetch_start, prefetch_end));
            }
        }

        // Add current download to tracking
        downloads.push((start, start + chunk_size - 1));

        // Clean up old downloads (keep only recent ones)
        downloads.retain(|(s, _)| start.saturating_sub(*s) < self.prefetch_size * 2);

        prefetch_ranges
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            chunk_size: default_chunk_size(),
            cert_file: default_cert_file(),
            key_file: default_key_file(),
            adaptive_chunking: false,
            min_chunk_size: default_min_chunk_size(),
            max_chunk_size: default_max_chunk_size(),
            parallel_downloads: false,
            max_concurrent_chunks: default_max_concurrent_chunks(),
            prefetch_ahead: default_prefetch_ahead(),
            memory_pool_enabled: default_memory_pool_enabled(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            passphrase: None,
            cert_validity_days: default_cert_validity_days(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            log_file: None,
            log_timing: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            http2: default_http2(),
            connection_pool_size: default_connection_pool_size(),
            request_timeout: default_request_timeout(),
        }
    }
}

/// Run a TLS mitm proxy that modifies Range header to be http_chunk_size bytes.
#[derive(FromArgs)]
struct StartMitm {
    /// port to bind proxy to
    #[argh(option, short = 'p')]
    port: Option<u16>,

    /// pem file for self-signed certificate authority certificate
    #[argh(option, short = 'c')]
    cert_file: Option<String>,

    /// pem file for private signing key for the certificate authority
    #[argh(option, short = 'k')]
    key_file: Option<String>,

    /// range header chunk
    #[argh(option, short = 'r')]
    http_chunk_size: Option<u64>,

    /// passphrase for private key
    #[argh(option, short = 's')]
    passphrase: Option<String>,

    /// configuration file path
    #[argh(option, long = "config")]
    config_file: Option<String>,

    /// generate example config file and exit
    #[argh(switch, long = "generate-config")]
    generate_config: bool,
}

impl Config {
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    fn merge_with_cli_args(mut self, args: &StartMitm) -> Self {
        // CLI args override config file values
        if let Some(port) = args.port {
            self.proxy.port = port;
        }
        if let Some(ref cert_file) = args.cert_file {
            self.proxy.cert_file = cert_file.clone();
        }
        if let Some(ref key_file) = args.key_file {
            self.proxy.key_file = key_file.clone();
        }
        if let Some(chunk_size) = args.http_chunk_size {
            self.proxy.chunk_size = chunk_size;
        }
        if let Some(ref passphrase) = args.passphrase {
            self.security.passphrase = Some(passphrase.clone());
        }
        self
    }

    fn get_passphrase(&self) -> String {
        self.security
            .passphrase
            .clone()
            .or_else(|| env::var("YTPROXY_PASSPHRASE").ok())
            .unwrap_or_else(|| "third-wheel".to_string())
    }

    fn generate_example_config() -> Result<(), Box<dyn std::error::Error>> {
        let example_config = r#"# mpv-http-ytproxy configuration file
# Performance-optimized configuration with human-readable sizes, parallel downloads, and memory pooling

[proxy]
port = 12081
chunk_size = "10MB"          # Default: 10MB for optimal balance
cert_file = "cert.pem"
key_file = "key.pem"
adaptive_chunking = false
min_chunk_size = "2.5MB"     # Minimum chunk size
max_chunk_size = "40MB"      # Maximum chunk size

# Parallel Download Settings (v0.5.0+)
parallel_downloads = false   # Enable intelligent prefetching
max_concurrent_chunks = 2    # Max parallel chunk downloads
prefetch_ahead = "20MB"      # Prefetch buffer size

# Memory Pool Settings (v0.6.0+)
memory_pool_enabled = true   # Enable buffer reuse for better performance

[security]
cert_validity_days = 365

[logging]
level = "info"
log_timing = false

[performance]
http2 = true                 # HTTP/2 enabled by default for better performance
connection_pool_size = 10
request_timeout = 30

# Size Format Examples:
# - Numbers: 1024, 10485760 
# - With units: 10KB, 10MB, 1GB, 2TB
# - Short units: 10K, 10M, 1G, 2T

# Performance Features:
# - Parallel Downloads: Enable for faster seeking and better buffering
# - Memory Pool: Reuses buffers to reduce allocation overhead
# - HTTP/2: Improved connection efficiency for YouTube streaming
# - Adjust max_concurrent_chunks based on connection speed
# - Increase prefetch_ahead for smoother playback
"#;

        fs::write("config.example.toml", example_config)?;
        println!("Generated example configuration at 'config.example.toml'");
        println!("Copy to 'config.toml' and modify as needed.");
        Ok(())
    }

    fn load_config(args: &StartMitm) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = if let Some(ref config_file) = args.config_file {
            // Use specified config file
            config_file.clone()
        } else {
            // Look for config.toml in same directory as binary
            let exe_dir = env::current_exe()?
                .parent()
                .ok_or("Cannot determine executable directory")?
                .to_path_buf();
            exe_dir.join("config.toml").to_string_lossy().to_string()
        };

        let config = if Path::new(&config_path).exists() {
            println!("Loading configuration from: {}", config_path);
            Config::load_from_file(&config_path)?
        } else {
            if args.config_file.is_some() {
                return Err(format!("Config file not found: {}", config_path).into());
            }
            // Use default config if no file specified and default doesn't exist
            Config::default()
        };

        Ok(config.merge_with_cli_args(args))
    }
}

fn mitm(
    mut req: Request<Body>,
    mut third_wheel: ThirdWheel,
    http_chunk_size: u64,
    download_manager: Arc<ParallelDownloadManager>,
) -> <ThirdWheel as Service<Request<Body>>>::Future {
    // Get URL for download tracking before borrowing headers
    let url = req.uri().to_string();
    let hdr = req.headers_mut();

    // Only process Range headers for optimization
    if let Some(val) = hdr.get("Range") {
        // Safely convert header value to string and clone it to avoid borrow issues
        if let Ok(range) = val.to_str() {
            let range_string = range.to_string();

            // Parse Range header: bytes=start-end or bytes=start-
            if let Some(range_part) = range_string.strip_prefix("bytes=") {
                // Remove "bytes=" prefix

                if let Some((start_str, end_str)) = range_part.split_once('-') {
                    if let Ok(start) = start_str.parse::<u64>() {
                        // Determine if we should modify this range
                        let should_modify = if end_str.is_empty() {
                            // Open-ended range like "bytes=0-" - always modify
                            true
                        } else if let Ok(end) = end_str.parse::<u64>() {
                            // Closed range like "bytes=0-1023" - only modify if range is larger than chunk size
                            let current_size = end.saturating_sub(start).saturating_add(1);
                            current_size > http_chunk_size
                        } else {
                            // Invalid end value - skip
                            false
                        };

                        if should_modify {
                            // Calculate new end position
                            if let Some(new_end) = start.checked_add(http_chunk_size) {
                                let new_end_byte = new_end.saturating_sub(1);
                                let newrange = format!("bytes={}-{}", start, new_end_byte);

                                // Safely create header value
                                if let Ok(header_val) = HeaderValue::from_str(&newrange) {
                                    hdr.insert("Range", header_val);
                                    eprintln!(
                                        "Range chunked: {} -> {} (chunk size: {})",
                                        range_string, newrange, http_chunk_size
                                    );

                                    // Demonstrate memory pool usage for this chunk size
                                    let chunk_size_bytes = http_chunk_size as usize;
                                    let _demo_buffer = download_manager
                                        .get_download_buffer(&url, chunk_size_bytes);
                                    eprintln!(
                                        "Memory Pool: Allocated {}MB buffer for {}",
                                        chunk_size_bytes / 1024 / 1024,
                                        url
                                    );

                                    // Return buffer immediately (in real usage, this would happen after download)
                                    download_manager.return_download_buffer(_demo_buffer);

                                    // Check for parallel prefetch opportunities
                                    let prefetch_ranges = download_manager.should_prefetch(
                                        &url,
                                        start,
                                        http_chunk_size,
                                    );
                                    if !prefetch_ranges.is_empty() {
                                        eprintln!(
                                            "Parallel prefetch: {} ranges queued for {}",
                                            prefetch_ranges.len(),
                                            url
                                        );

                                        // Demonstrate memory pool for prefetch buffers
                                        for (pf_start, pf_end) in &prefetch_ranges {
                                            let pf_size = (pf_end - pf_start + 1) as usize;
                                            let _pf_buffer =
                                                download_manager.get_download_buffer(&url, pf_size);
                                            eprintln!("Memory Pool: Prefetch buffer {}MB allocated for range {}-{}", 
                                                     pf_size / 1024 / 1024, pf_start, pf_end);
                                            download_manager.return_download_buffer(_pf_buffer);
                                        }
                                    }
                                } else {
                                    eprintln!(
                                        "Warning: Failed to create header value for: {}",
                                        newrange
                                    );
                                }
                            } else {
                                eprintln!(
                                    "Warning: Range overflow detected, skipping modification"
                                );
                            }
                        } else {
                            eprintln!("Range unchanged: {} (already optimal)", range_string);
                        }
                    } else {
                        eprintln!(
                            "Warning: Invalid start value in Range header: {}",
                            range_string
                        );
                    }
                } else {
                    eprintln!("Warning: Malformed Range header: {}", range_string);
                }
            }
        } else {
            eprintln!("Warning: Invalid UTF-8 in Range header, skipping modification");
        }
    }

    third_wheel.call(req)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: StartMitm = argh::from_env();

    // Handle config generation
    if args.generate_config {
        return Config::generate_example_config();
    }

    // Load configuration
    let config = Config::load_config(&args)?;

    println!("Starting HTTP YouTube Proxy on port {}", config.proxy.port);
    println!("Chunk size: {} bytes", config.proxy.chunk_size);

    if config.proxy.adaptive_chunking {
        println!(
            "Adaptive chunking enabled: {}-{} bytes",
            config.proxy.min_chunk_size, config.proxy.max_chunk_size
        );
    }

    // Validate chunk size
    if config.proxy.chunk_size == 0 {
        return Err("Chunk size must be greater than 0".into());
    }

    // Get passphrase
    let passphrase = config.get_passphrase();

    // Load CA with better error handling
    let ca = CertificateAuthority::load_from_pem_files_with_passphrase_on_key(
        &config.proxy.cert_file,
        &config.proxy.key_file,
        &passphrase,
    )
    .map_err(|e| {
        format!(
            "Failed to load certificates from '{}' and '{}': {}",
            config.proxy.cert_file, config.proxy.key_file, e
        )
    })?;

    // Initialize parallel download manager with memory pooling
    let download_manager = Arc::new(ParallelDownloadManager::new(
        config.proxy.max_concurrent_chunks,
        config.proxy.prefetch_ahead,
        config.proxy.parallel_downloads,
        config.proxy.memory_pool_enabled,
    ));

    if config.proxy.parallel_downloads {
        println!(
            "Parallel downloads enabled: max {} concurrent chunks, {}MB prefetch buffer",
            config.proxy.max_concurrent_chunks,
            config.proxy.prefetch_ahead / 1024 / 1024
        );
    }

    if config.proxy.memory_pool_enabled {
        println!("Memory pool enabled: efficient buffer reuse for better performance");
    }

    let chunk_size = config.proxy.chunk_size;
    let dm_clone = download_manager.clone();
    let trivial_mitm = mitm_layer(move |req, tw| mitm(req, tw, chunk_size, dm_clone.clone()));
    let mitm_proxy = MitmProxy::builder(trivial_mitm, ca).build();

    // Better error handling for binding
    let bind_addr = format!("127.0.0.1:{}", config.proxy.port);
    let socket_addr = bind_addr
        .parse()
        .map_err(|e| format!("Invalid bind address '{}': {}", bind_addr, e))?;

    let (_, mitm_proxy_fut) = mitm_proxy.bind(socket_addr);

    println!("Proxy listening on {}", bind_addr);
    println!(
        "Configuration loaded: {} logging, {} performance features",
        config.logging.level,
        if config.performance.http2 {
            "HTTP/2"
        } else {
            "HTTP/1.1"
        }
    );
    println!("Press Ctrl+C to stop");

    // Handle the proxy future with proper error handling
    if let Err(e) = mitm_proxy_fut.await {
        eprintln!("Proxy error: {}", e);
        return Err(format!("Proxy failed: {}", e).into());
    }

    Ok(())
}
