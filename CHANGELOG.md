# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-01-01

### Added

- Initial release of Axiom framework
- Procedural macros for API definition
- HTTP protocol support with Axum 0.8.8
- MCP protocol support with mcp-sdk 0.0.3
- Feature-gated code generation
- Automatic service discovery via inventory
- Module-level path prefixes
- Version management
- Core types: ApiMetadata, ServiceResponse, ApiError, ServiceError
- Timestamp feature support
- Logging feature support
- Streaming feature support (SSE)
- Input validation utilities
- Configuration management
- Comprehensive test suite
- Performance benchmarks
- Documentation and examples

### Features

- **Unified Interface**: Single `#[service_api]` macro for both HTTP and MCP
- **Compile-Time Protocol Selection**: Features control which protocols are compiled
- **Zero Overhead**: Unused protocols don't appear in the binary
- **Type Safety**: Compile-time validation of API configurations

### Supported Protocols

- HTTP (via Axum)
- MCP (via mcp-sdk)

### Supported Features

- `http`: HTTP server support
- `mcp`: MCP protocol support
- `streaming`: SSE streaming
- `timestamp`: Automatic response timestamps
- `logging`: Structured request logging
- `full`: All features

### Examples

- Basic HTTP CRUD API
- Complex type handling
- Module prefixes
- Multi-version APIs

### Testing

- HTTP integration tests
- Complex type serialization tests
- Feature combination tests
- Performance benchmarks
