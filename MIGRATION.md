# Configuration Migration Guide

## Overview

This guide helps you migrate your SDForge configuration files to work with the updated configuration structure introduced in version 0.2.

## What Changed?

### Authentication Configuration

The `auth` field has been renamed to `authentication`, and the authentication configuration structure has changed from a simple struct to an enum that supports multiple authentication strategies.

#### Old Format (v0.1)

```toml
[auth]
jwt_secret = "your-secret-key"
token_expiry = 3600
```

#### New Format (v0.2+)

```toml
[authentication]
type = "jwt"
secret = "your-secret-key"
```

**Note**: The `token_expiry` field has been removed. Token expiry should be configured through the authentication middleware instead.

## Migration Steps

### Step 1: Update Authentication Configuration

#### For JWT Authentication

**Before:**
```toml
[auth]
jwt_secret = "my-secret-key"
token_expiry = 3600
```

**After:**
```toml
[authentication]
type = "jwt"
secret = "my-secret-key"
```

#### For API Key Authentication

**Before:** (Not supported in v0.1)

**After:**
```toml
[authentication]
type = "api_key"
header_name = "X-API-Key"
prefix = "Key "
```

### Step 2: Add Rate Limiting Configuration (Optional)

If you want to enable rate limiting, add the following section:

```toml
[rate_limit]
requests = 100
window_seconds = 60
```

### Step 3: Verify Your Configuration

After making changes, verify your configuration:

```bash
# Check if the configuration is valid
cargo run --bin sdforge -- validate-config --path config.toml

# Test with a specific feature set
cargo run --features "http,security" -- --config config.toml
```

## Complete Example Configuration

Here's a complete example configuration file that incorporates all the changes:

```toml
# Server configuration
[server]
host = "0.0.0.0"
port = 3000
request_timeout_secs = 30

# CORS configuration (optional)
[server.cors]
allowed_origins = ["http://localhost:3000", "https://example.com"]
allowed_methods = ["GET", "POST", "PUT", "DELETE"]
allowed_headers = ["Content-Type", "Authorization"]

# Database configuration
[database]
connection_string = "postgresql://user:password@localhost/database"
max_connections = 10

# Authentication configuration
[authentication]
type = "jwt"
secret = "your-jwt-secret-key-here"

# Alternative: API Key authentication
# [authentication]
# type = "api_key"
# header_name = "X-API-Key"
# prefix = "Key "

# Rate limiting configuration (optional)
[rate_limit]
requests = 100
window_seconds = 60

# Logging configuration
[logging]
level = "info"
format = "json"
```

## Backward Compatibility

The new configuration structure maintains backward compatibility with the old `auth` field name through serde aliases. This means:

- Old configurations using `[auth]` will still work
- New configurations should use `[authentication]`
- A warning will be logged if the old format is detected

## Troubleshooting

### Error: "Unknown authentication type"

**Problem**: The `type` field is missing or has an invalid value.

**Solution**: Ensure you have specified a valid `type`:
- `jwt` - For JWT token authentication
- `api_key` - For API key authentication
- `oauth2` - For OAuth2 (not yet implemented)

### Error: "Field 'jwt_secret' not found"

**Problem**: You're using the old authentication format.

**Solution**: Update your configuration to use the new enum-based format as shown in Step 1.

### Warning: "Using deprecated 'auth' field"

**Problem**: Your configuration uses the old `auth` field name.

**Solution**: Rename `[auth]` to `[authentication]` in your configuration file.

## Rollback

If you encounter issues and need to rollback to the previous version:

1. Revert your configuration file to the old format
2. Use SDForge v0.1.x
3. Report the issue at: https://github.com/sdforge-rs/sdforge/issues

## Need Help?

If you need assistance with migration:

- Check the [Documentation](https://docs.rs/sdforge)
- Open an issue on [GitHub](https://github.com/sdforge-rs/sdforge/issues)
- Join our community discussions

## Migration Checklist

- [ ] Update `[auth]` to `[authentication]`
- [ ] Add `type` field to authentication configuration
- [ ] Update authentication-specific fields (`jwt_secret` → `secret`)
- [ ] Remove deprecated `token_expiry` field
- [ ] Add `[rate_limit]` section if needed
- [ ] Test your configuration with `--validate-config`
- [ ] Update your deployment scripts and documentation
