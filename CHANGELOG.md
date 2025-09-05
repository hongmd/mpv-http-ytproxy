# Changelog

All notable changes to this project will be documented in this file.

## [0.5.0] - 2025-09-05

### Added
- **Parallel Download Support** - Intelligent prefetching for better performance
  - `parallel_downloads` setting to enable/disable parallel chunk downloads
  - `max_concurrent_chunks` to control concurrent download limit  
  - `prefetch_ahead` buffer size for intelligent prefetching
  - Download manager to track and coordinate parallel requests
  - Automatic prefetch range calculation for optimal buffering
- **Enhanced Download Intelligence** - Smart prefetching based on current position
- **Better Performance Logging** - Detailed information about parallel download activity

### Changed
- MITM function now supports parallel download coordination
- Configuration structure extended with parallel download settings
- Default configuration includes parallel download options (disabled by default)
- Improved memory management for download tracking

### Technical Details
- Parallel downloads use conservative defaults (2 concurrent, 20MB prefetch)
- Download manager prevents duplicate range requests
- Automatic cleanup of completed download tracking
- Foundation for future adaptive prefetching algorithms

## [0.4.0] - 2025-09-05

### Added
- **Human-Readable Size Format** - Support for intuitive size notation in config
  - Use `"50MB"`, `"1GB"`, `"512KB"` instead of byte numbers
  - Support both long (`MB`, `GB`) and short (`M`, `G`) unit formats
  - Backward compatible with numeric byte values
  - Custom deserializer with comprehensive error handling
- **Anti-Rate Limiting** - Default 10MB chunks with option to increase for faster connections
- **Enhanced Documentation** - Clear examples of size format usage in config files

### Changed
- Default `chunk_size` set to 10MB for optimal balance of performance and bandwidth
- **HTTP/2 enabled by default** for better streaming performance
- **Removed compression setting** (unnecessary for pre-compressed video content)
- Config files now use human-readable format by default
- Improved config generation with user-friendly size examples

### Fixed
- YouTube HTTP 429 "Too Many Requests" errors through larger chunk sizes
- Better balance between performance and API compliance

## [0.3.0] - 2025-09-05

### Added
- **Configuration File Support** - TOML-based configuration with CLI override
  - Generate example config with `--generate-config`
  - Load config from `config.toml` or custom path with `--config`
  - Support for all proxy, security, logging, and performance settings
- **Enhanced CLI Arguments** - All arguments now optional with config file fallbacks
- **Improved Lua Script** - Uses configuration file instead of hardcoded values
- **Better Build Script** - Automatically generates and copies config file
- **Enhanced Error Handling** - Comprehensive error messages and validation
- **Structured Configuration** - Organized settings into logical sections

### Changed
- Default configuration uses config file instead of hardcoded CLI arguments
- Lua script now passes `--config` instead of individual parameters
- Build script automatically sets up configuration for users
- Version bumped to 0.3.0 to reflect major feature addition

### Fixed
- Lua script properly uses variable chunk sizes from configuration
- Better certificate path handling in configuration
- Improved error messages for missing files

## [0.2.0] - 2025-09-05

### Added
- **Smart Range Header Processing** - Intelligent chunking based on request type
- **Better Error Handling** - Eliminated all `unwrap()` calls for safer operation
- **Enhanced Logging** - Detailed logging with chunk size information
- **Security Improvements** - Configurable passphrase support
- **Input Validation** - Comprehensive validation for all user inputs
- **Cross-platform Build** - Improved build script for macOS/Linux

### Changed
- Range header modification logic now handles open-ended and closed ranges differently
- Only modifies ranges larger than chunk size for efficiency
- Better borrow checker compliance in Rust code
- Enhanced Lua script with improved URL validation

### Fixed
- Borrow checker issues in Range header processing
- Integer overflow protection in chunk size calculations
- Proper UTF-8 handling for Range headers

## [0.1.0] - Initial Release

### Added
- Basic HTTP MITM proxy for YouTube streaming optimization
- Range header modification for 10MB chunks
- mpv integration via Lua script
- TLS certificate generation and management
- Basic build and installation scripts

---

### Legend
- **Added** for new features
- **Changed** for changes in existing functionality  
- **Deprecated** for soon-to-be removed features
- **Removed** for now removed features
- **Fixed** for any bug fixes
- **Security** for vulnerability fixes
