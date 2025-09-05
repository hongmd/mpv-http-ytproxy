use argh::FromArgs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use third_wheel::hyper::{Request, Body};
use third_wheel::hyper::http::HeaderValue;
use third_wheel::hyper::service::Service;
use third_wheel::*;

#[derive(Debug, Deserialize, Serialize)]
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
    #[serde(default = "default_chunk_size")]
    chunk_size: u64,
    #[serde(default = "default_cert_file")]
    cert_file: String,
    #[serde(default = "default_key_file")]
    key_file: String,
    #[serde(default)]
    adaptive_chunking: bool,
    #[serde(default = "default_min_chunk_size")]
    min_chunk_size: u64,
    #[serde(default = "default_max_chunk_size")]
    max_chunk_size: u64,
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
    #[serde(default)]
    http2: bool,
    #[serde(default = "default_connection_pool_size")]
    connection_pool_size: u32,
    #[serde(default = "default_request_timeout")]
    request_timeout: u64,
    #[serde(default)]
    enable_compression: bool,
}

// Default value functions
fn default_port() -> u16 { 8080 }
fn default_chunk_size() -> u64 { 10485760 } // 10MB
fn default_cert_file() -> String { "cert.pem".to_string() }
fn default_key_file() -> String { "key.pem".to_string() }
fn default_min_chunk_size() -> u64 { 2621440 } // 2.5MB
fn default_max_chunk_size() -> u64 { 41943040 } // 40MB
fn default_cert_validity_days() -> u32 { 365 }
fn default_log_level() -> String { "info".to_string() }
fn default_connection_pool_size() -> u32 { 10 }
fn default_request_timeout() -> u64 { 30 }

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
            http2: false,
            connection_pool_size: default_connection_pool_size(),
            request_timeout: default_request_timeout(),
            enable_compression: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
            performance: PerformanceConfig::default(),
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
        self.security.passphrase
            .clone()
            .or_else(|| env::var("YTPROXY_PASSPHRASE").ok())
            .unwrap_or_else(|| "third-wheel".to_string())
    }

    fn generate_example_config() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::default();
        let toml_string = toml::to_string_pretty(&config)?;
        
        let example_config = format!(
            "# mpv-http-ytproxy configuration file\n\
             # Place this file as 'config.toml' in the same directory as the binary\n\
             # or specify with --config /path/to/config.toml\n\n\
             {}", 
            toml_string
        );
        
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


fn mitm(mut req: Request<Body>, mut third_wheel: ThirdWheel, http_chunk_size: u64) -> <ThirdWheel as Service<Request<Body>>>::Future {
    let hdr = req.headers_mut();
    
    // Only process Range headers for optimization
    if let Some(val) = hdr.get("Range") {
        // Safely convert header value to string and clone it to avoid borrow issues
        if let Ok(range) = val.to_str() {
            let range_string = range.to_string();
            
            // Parse Range header: bytes=start-end or bytes=start-
            if range_string.starts_with("bytes=") {
                let range_part = &range_string[6..]; // Remove "bytes=" prefix
                
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
                                    eprintln!("Range chunked: {} -> {} (chunk size: {})", 
                                             range_string, newrange, http_chunk_size);
                                } else {
                                    eprintln!("Warning: Failed to create header value for: {}", newrange);
                                }
                            } else {
                                eprintln!("Warning: Range overflow detected, skipping modification");
                            }
                        } else {
                            eprintln!("Range unchanged: {} (already optimal)", range_string);
                        }
                    } else {
                        eprintln!("Warning: Invalid start value in Range header: {}", range_string);
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
        println!("Adaptive chunking enabled: {}-{} bytes", 
                config.proxy.min_chunk_size, config.proxy.max_chunk_size);
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
    ).map_err(|e| {
        format!("Failed to load certificates from '{}' and '{}': {}", 
                config.proxy.cert_file, config.proxy.key_file, e)
    })?;
    
    let chunk_size = config.proxy.chunk_size;
    let trivial_mitm = mitm_layer(move |req, tw| mitm(req, tw, chunk_size));
    let mitm_proxy = MitmProxy::builder(trivial_mitm, ca).build();
    
    // Better error handling for binding
    let bind_addr = format!("127.0.0.1:{}", config.proxy.port);
    let socket_addr = bind_addr.parse()
        .map_err(|e| format!("Invalid bind address '{}': {}", bind_addr, e))?;
    
    let (_, mitm_proxy_fut) = mitm_proxy.bind(socket_addr);
    
    println!("Proxy listening on {}", bind_addr);
    println!("Configuration loaded: {} logging, {} performance features", 
             config.logging.level, 
             if config.performance.http2 { "HTTP/2" } else { "HTTP/1.1" });
    println!("Press Ctrl+C to stop");
    
    // Handle the proxy future with proper error handling
    if let Err(e) = mitm_proxy_fut.await {
        eprintln!("Proxy error: {}", e);
        return Err(format!("Proxy failed: {}", e).into());
    }
    
    Ok(())
}
