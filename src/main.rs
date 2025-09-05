use argh::FromArgs;
use std::env;
use third_wheel::hyper::{Request, Body};
use third_wheel::hyper::http::HeaderValue;
use third_wheel::hyper::service::Service;
use third_wheel::*;

/// Run a TLS mitm proxy that modifies Range header to be http_chunk_size bytes.
#[derive(FromArgs)]
struct StartMitm {
    /// port to bind proxy to
    #[argh(option, short = 'p', default = "8080")]
    port: u16,

    /// pem file for self-signed certificate authority certificate
    #[argh(option, short = 'c', default = "\"cert.pem\".to_string()")]
    cert_file: String,

    /// pem file for private signing key for the certificate authority
    #[argh(option, short = 'k', default = "\"key.pem\".to_string()")]
    key_file: String,

    /// range header chunk
    #[argh(option, short = 'r', default = "10485760")]
    http_chunk_size: u64,

    /// passphrase for private key (defaults to env var YTPROXY_PASSPHRASE or "third-wheel")
    #[argh(option, short = 's')]
    passphrase: Option<String>,
}


fn mitm(mut req: Request<Body>, mut third_wheel: ThirdWheel, http_chunk_size: u64) -> <ThirdWheel as Service<Request<Body>>>::Future {
    let hdr = req.headers_mut();
    
    // Only process Range headers for optimization
    if let Some(val) = hdr.get("Range") {
        // Safely convert header value to string and clone it to avoid borrow issues
        if let Ok(range) = val.to_str() {
            let range_string = range.to_string(); // Clone to avoid borrow checker issues
            if range_string.starts_with("bytes=") {
                if let Some((start_str, _end_str)) = range_string[6..].split_once('-') {
                    if let Ok(start) = start_str.parse::<u64>() {
                        // Check for overflow before addition
                        if let Some(end) = start.checked_add(http_chunk_size) {
                            let newrange = format!("bytes={}-{}", start, end.saturating_sub(1));
                            // Safely create header value
                            if let Ok(header_val) = HeaderValue::from_str(&newrange) {
                                hdr.insert("Range", header_val);
                                eprintln!("Range modified: {} -> {}", range_string, newrange);
                            } else {
                                eprintln!("Warning: Failed to create header value for: {}", newrange);
                            }
                        } else {
                            eprintln!("Warning: Range overflow detected, skipping modification");
                        }
                    }
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
    
    // Get passphrase from args, env var, or default
    let passphrase = args.passphrase
        .or_else(|| env::var("YTPROXY_PASSPHRASE").ok())
        .unwrap_or_else(|| "third-wheel".to_string());
    
    println!("Starting HTTP YouTube Proxy on port {}", args.port);
    println!("Chunk size: {} bytes", args.http_chunk_size);
    
    // Validate chunk size
    if args.http_chunk_size == 0 {
        return Err("Chunk size must be greater than 0".into());
    }
    
    // Load CA with better error handling
    let ca = CertificateAuthority::load_from_pem_files_with_passphrase_on_key(
        &args.cert_file,
        &args.key_file,
        &passphrase,
    ).map_err(|e| {
        format!("Failed to load certificates from '{}' and '{}': {}", 
                args.cert_file, args.key_file, e)
    })?;
    
    let trivial_mitm = mitm_layer(move |req, tw| mitm(req, tw, args.http_chunk_size));
    let mitm_proxy = MitmProxy::builder(trivial_mitm, ca).build();
    
    // Better error handling for binding
    let bind_addr = format!("127.0.0.1:{}", args.port);
    let socket_addr = bind_addr.parse()
        .map_err(|e| format!("Invalid bind address '{}': {}", bind_addr, e))?;
    
    let (_, mitm_proxy_fut) = mitm_proxy.bind(socket_addr);
    
    println!("Proxy listening on {}", bind_addr);
    println!("Press Ctrl+C to stop");
    
    // Handle the proxy future with proper error handling
    if let Err(e) = mitm_proxy_fut.await {
        eprintln!("Proxy error: {}", e);
        return Err(format!("Proxy failed: {}", e).into());
    }
    
    Ok(())
}
