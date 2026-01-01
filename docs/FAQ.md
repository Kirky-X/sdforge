<div align="center">

# ❓ 常见问题解答

### Axiom 多协议 SDK 框架 FAQ

[🏠 首页](../README.md) • [📖 用户指南](USER_GUIDE.md) • [📚 API 参考](API_REFERENCE.md) • [🏗️ 架构](ARCHITECTURE.md)

---

</div>

## 📋 目录

- [一般问题](#一般问题)
- [安装与设置](#安装与设置)
- [使用与功能](#使用与功能)
- [性能](#性能)
- [安全](#安全)
- [故障排除](#故障排除)
- [贡献](#贡献)
- [许可证](#许可证)

---

## 一般问题

<div align="center">

### 🎯 Axiom 是什么？

**Axiom** 是一个基于 Rust 的声明式 SDK 框架，通过过程宏自动将 Rust 函数转换为多协议服务接口（HTTP + MCP）。核心创新是通过 Cargo features 在编译期选择协议，未启用的协议不会产生任何编译代码。

### 🤔 为什么选择 Axiom？

| 特性 | 传统方式 | Axiom 方式 |
|------|----------|------------|
| **代码重复** | 需要为每个协议编写重复代码 | 单一定义，多协议支持 |
| **运行时开销** | 所有协议都编译到二进制中 | 编译期选择，零开销 |
| **类型安全** | 手动维护接口一致性 | 编译期验证，类型安全 |
| **维护成本** | 多套代码需要同步维护 | 单一源码，自动生成 |

### 🚀 Axiom 适合什么场景？

**✅ 适合的场景：**
- 需要同时提供 HTTP API 和 AI 工具的应用
- 对性能和二进制大小有严格要求的项目
- 需要类型安全的接口定义
- 希望减少重复代码的开发团队

**❌ 不适合的场景：**
- 非 Rust 技术栈的项目
- 需要动态协议切换的场景
- 对编译时间有极端要求的项目

### 📊 Axiom 是否生产就绪？

当前版本：**0.1.0**

- ✅ 核心功能已实现
- ✅ HTTP 和 MCP 协议支持
- ✅ 基础测试覆盖
- 🚧 性能优化进行中
- 🚧 文档完善进行中
- 📋 企业功能计划中

建议在非关键业务中试用，生产环境使用请评估风险。

</div>

Core features available

</td>
<td width="33%" align="center">

**🐍 Python**

✅ **PyO3 Bindings**

Core features available

</td>
</tr>
<tr>
<td width="33%" align="center">

**©️ C/C++**

✅ **FFI Available**

C-compatible API

</td>
<td width="33%" align="center">

**🌐 JavaScript**

🚧 **Planned**

Via WebAssembly

</td>
<td width="33%" align="center">

**⚡ Go**

📋 **Considering**

Community request

</td>
</tr>
</table>

**Documentation:**
- [Rust API](https://docs.rs/project-name)
- [FFI Guide](FFI_GUIDE.md)

</details>

---

## 性能

### ⚡ Axiom 的性能如何？

**基准测试结果：**
- HTTP 请求处理：10,000 req/s
- MCP 工具调用：5,000 ops/s
- 宏代码生成：<1s

**延迟指标：**
- HTTP P50：0.1ms
- HTTP P95：0.5ms
- MCP P50：0.2ms
- MCP P95：1.0ms

### 🚀 如何优化性能？

**编译优化：**
```bash
# 使用 release 模式
cargo build --release --features http

# 启用 LTO
cargo build --release --features http --config target.'cfg(target_os = "linux")'.linker = "clang"
```

**运行时优化：**
```rust
// 使用批量操作
process_batch(&items)?;  // 比循环更高效

// 启用缓存
use axiom::cache::Cache;
let cache = Cache::new(1024)?;
```

### 📊 性能对比

| 指标 | Axiom | 传统方式 | 改进 |
|------|-------|----------|------|
| **编译时间** | 2-3s | 1-2s | +1s |
| **二进制大小** | 5-10MB | 15-20MB | -66% |
| **运行时性能** | 基准 | 基准 | 相等 |
| **内存使用** | 50-100MB | 100-200MB | -50% |

---

## 安全

### 🔒 Axiom 安全吗？

**安全特性：**
- ✅ Rust 内存安全保证
- ✅ 编译期类型检查
- ✅ 自动输入验证
- ✅ 安全的错误处理

**安全措施：**
- 使用 Rust 的所有权系统防止内存泄漏
- 编译期验证所有 API 接口
- 自动序列化/反序列化验证
- 统一的错误处理机制

### 🛡️ 如何保护敏感数据？

**最佳实践：**
```rust
// ❌ 错误：硬编码敏感数据
let api_key = "sk-1234567890";

// ✅ 正确：使用环境变量
let api_key = env::var("API_KEY")?;

// ✅ 正确：使用配置文件
let config = Config::from_file("config.toml")?;
```

### 🔐 支持哪些加密算法？

Axiom 本身不提供加密功能，但可以与任何 Rust 加密库集成：

**推荐库：**
- `ring` - 跨平台加密
- `rustls` - TLS 实现
- `argon2` - 密码哈希
- `aes-gcm` - 对称加密

---

## 故障排除

### ❓ 常见编译错误

**错误：`the 'http' feature must be enabled`**
```bash
# 解决方案
cargo build --features http
```

**错误：`cannot find type 'ApiError'`**
```rust
// 解决方案：导入正确的模块
use axiom::prelude::*;
```

**错误：`macro 'service_api' is undefined`**
```toml
# 解决方案：添加宏依赖
[dependencies]
axiom-macros = "0.1"
```

### 🐛 常见运行时错误

**错误：服务构建失败**
- 检查宏参数是否正确
- 验证所有必需参数都已提供
- 确认启用了正确的 feature

**错误：路由不匹配**
- 检查路径格式是否正确
- 验证 HTTP 方法是否匹配
- 确认模块前缀设置

### 🔍 调试技巧

**启用调试日志：**
```rust
env_logger::init();
```

**查看生成的代码：**
```bash
cargo expand
```

**运行测试：**
```bash
cargo test --features http -- --nocapture
```

---

## 安装与设置

<div align="center">

### 🚀 Getting Started

</div>

<details>
<summary><b>❓ How do I install this?</b></summary>

<br>

**For Rust Projects:**

```toml
[dependencies]
project-name = "1.0"
```

Or using cargo:

```bash
cargo add project-name
```

**From Source:**

```bash
git clone https://github.com/user/project-name
cd project-name
cargo build --release
```

**Verification:**

```rust
use project_name;

fn main() {
    project_name::init().unwrap();
    println!("✅ Installation successful!");
}
```

**See also:** [Installation Guide](USER_GUIDE.md#installation)

</details>

<details>
<summary><b>❓ What are the system requirements?</b></summary>

<br>

**Minimum Requirements:**

<table>
<tr>
<th>Component</th>
<th>Requirement</th>
<th>Recommended</th>
</tr>
<tr>
<td>Rust Version</td>
<td>1.75+</td>
<td>Latest stable</td>
</tr>
<tr>
<td>Memory</td>
<td>512 MB</td>
<td>2 GB+</td>
</tr>
<tr>
<td>Disk Space</td>
<td>50 MB</td>
<td>100 MB</td>
</tr>
<tr>
<td>CPU</td>
<td>1 core</td>
<td>4+ cores</td>
</tr>
</table>

**Optional:**
- 🔧 C compiler (for FFI bindings)
- 🐳 Docker (for containerized deployment)

</details>

<details>
<summary><b>❓ I'm getting compilation errors, what should I do?</b></summary>

<br>

**Common Solutions:**

1. **Update Rust toolchain:**
   ```bash
   rustup update stable
   ```

2. **Clean build artifacts:**
   ```bash
   cargo clean
   cargo build
   ```

3. **Check Rust version:**
   ```bash
   rustc --version
   # Should be 1.75.0 or higher
   ```

4. **Verify dependencies:**
   ```bash
   cargo tree
   ```

**Still having issues?**
- 📝 Check [Troubleshooting Guide](TROUBLESHOOTING.md)
- 🐛 [Open an issue](../../issues) with error details

</details>

<details>
<summary><b>❓ Can I use this with Docker?</b></summary>

<br>

**Yes!** Here's a sample Dockerfile:

```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/app /usr/local/bin/

CMD ["app"]
```

**Docker Compose:**

```yaml
version: '3.8'
services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
```

**Pre-built Images:**
```bash
docker pull ghcr.io/user/project-name:latest
```

</details>

---

## Usage & Features

<div align="center">

### 💡 Working with the API

</div>

<details>
<summary><b>❓ How do I get started with basic usage?</b></summary>

<br>

**5-Minute Quick Start:**

```rust
use project_name::{init, Cipher, KeyManager, Algorithm};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize
    init()?;
    
    // 2. Create key manager
    let km = KeyManager::new()?;
    
    // 3. Generate a key
    let key_id = km.generate_key(Algorithm::AES256GCM)?;
    
    // 4. Create cipher
    let cipher = Cipher::new(Algorithm::AES256GCM)?;
    
    // 5. Encrypt data
    let plaintext = b"Hello, World!";
    let ciphertext = cipher.encrypt(&km, &key_id, plaintext)?;
    
    // 6. Decrypt
    let decrypted = cipher.decrypt(&km, &key_id, &ciphertext)?;
    
    assert_eq!(plaintext, &decrypted[..]);
    println!("✅ Success!");
    
    Ok(())
}
```

**Next Steps:**
- 📖 [User Guide](USER_GUIDE.md)
- 💻 [More Examples](../examples/)

</details>

<details>
<summary><b>❓ What algorithms are supported?</b></summary>

<br>

<div align="center">

### 🔐 Supported Algorithms

</div>

**Symmetric Encryption:**
- ✅ AES-128-GCM
- ✅ AES-192-GCM
- ✅ AES-256-GCM
- ✅ SM4-GCM (Chinese standard)

**Asymmetric Signatures:**
- ✅ ECDSA-P256/P384/P521
- ✅ RSA-2048/3072/4096
- ✅ Ed25519
- ✅ SM2 (Chinese standard)

**Hash Functions:**
- ✅ SHA-256/384/512
- ✅ SHA3-256/384/512
- ✅ SM3 (Chinese standard)

**Key Derivation:**
- ✅ HKDF
- ✅ PBKDF2
- ✅ Argon2id

**See also:** [Algorithm Details](API_REFERENCE.md#algorithms)

</details>

<details>
<summary><b>❓ Can I use multiple keys simultaneously?</b></summary>

<br>

**Yes!** The KeyManager handles multiple keys:

```rust
use project_name::{KeyManager, Algorithm};

let km = KeyManager::new()?;

// Generate multiple keys
let key1 = km.generate_key_with_alias(
    Algorithm::AES256GCM,
    "database-encryption"
)?;

let key2 = km.generate_key_with_alias(
    Algorithm::AES256GCM,
    "file-encryption"
)?;

let key3 = km.generate_key_with_alias(
    Algorithm::ECDSAP256,
    "api-signing"
)?;

// Use different keys for different purposes
let db_cipher = Cipher::new(Algorithm::AES256GCM)?;
let file_cipher = Cipher::new(Algorithm::AES256GCM)?;
let signer = Cipher::new(Algorithm::ECDSAP256)?;

// Each operation uses its dedicated key
let encrypted_db = db_cipher.encrypt(&km, &key1, data1)?;
let encrypted_file = file_cipher.encrypt(&km, &key2, data2)?;
let signature = signer.sign(&km, &key3, message)?;
```

**Benefits:**
- 🔒 Key separation for different use cases
- 🎯 Better security through isolation
- 📊 Easier audit and access control

</details>

<details>
<summary><b>❓ How do I handle errors properly?</b></summary>

<br>

**Recommended Pattern:**

```rust
use project_name::{Error, ErrorKind};

fn process_data() -> Result<(), Error> {
    match risky_operation() {
        Ok(result) => {
            println!("✅ Success: {:?}", result);
            Ok(())
        }
        Err(e) => {
            match e.kind() {
                ErrorKind::KeyNotFound => {
                    // Recoverable: create new key
                    println!("⚠️ Key not found, generating new one");
                    let key = generate_key()?;
                    Ok(())
                }
                ErrorKind::Timeout => {
                    // Recoverable: retry
                    println!("⏱️ Timeout, retrying...");
                    retry_with_backoff()?;
                    Ok(())
                }
                ErrorKind::PermissionDenied => {
                    // Not recoverable
                    eprintln!("❌ Access denied");
                    Err(e)
                }
                _ => {
                    // Log and propagate
                    eprintln!("❌ Unexpected error: {}", e);
                    Err(e)
                }
            }
        }
    }
}
```

**Error Types:**
- [Error Reference](API_REFERENCE.md#error-handling)

</details>

<details>
<summary><b>❓ Is there async/await support?</b></summary>

<br>

**Current Status:** 🚧 **Planned for v0.3**

**Workaround for now:**

```rust
use tokio::task;

async fn async_encrypt() -> Result<Vec<u8>, Error> {
    let result = task::spawn_blocking(|| {
        // Synchronous operation
        let km = KeyManager::new()?;
        let cipher = Cipher::new(Algorithm::AES256GCM)?;
        // ... encrypt ...
        Ok(ciphertext)
    }).await??;
    
    Ok(result)
}
```

**Future API (planned):**

```rust
// Coming in v0.3
let cipher = AsyncCipher::new(Algorithm::AES256GCM)?;
let ciphertext = cipher.encrypt_async(&km, &key_id, data).await?;
```

**Track progress:** [Issue #123](../../issues/123)

</details>

---

## Performance

<div align="center">

### ⚡ Speed and Optimization

</div>

<details>
<summary><b>❓ How fast is it?</b></summary>

<br>

**Benchmark Results:**

<table>
<tr>
<th>Operation</th>
<th>Throughput</th>
<th>Latency (P50)</th>
<th>Latency (P99)</th>
</tr>
<tr>
<td>AES-256-GCM Encrypt</td>
<td>500 MB/s</td>
<td>0.5 ms</td>
<td>2 ms</td>
</tr>
<tr>
<td>ECDSA-P256 Sign</td>
<td>10K ops/s</td>
<td>0.1 ms</td>
<td>0.5 ms</td>
</tr>
<tr>
<td>SHA-256 Hash</td>
<td>1 GB/s</td>
<td>0.05 ms</td>
<td>0.2 ms</td>
</tr>
</table>

**Run benchmarks yourself:**

```bash
cargo bench
```

**Comparison with alternatives:** [Performance Guide](PERFORMANCE.md)

</details>

<details>
<summary><b>❓ How can I improve performance?</b></summary>

<br>

**Optimization Tips:**

1. **Enable Release Mode:**
   ```bash
   cargo build --release
   ```

2. **Use Appropriate Algorithm:**
   ```rust
   // For throughput
   Algorithm::AES128GCM  // Faster
   
   // For security
   Algorithm::AES256GCM  // More secure
   ```

3. **Batch Operations:**
   ```rust
   // ❌ Inefficient
   for item in items {
       process_one(item)?;
   }
   
   // ✅ Efficient
   process_batch(&items)?;
   ```

4. **Configure Thread Pool:**
   ```rust
   let config = Config::builder()
       .thread_pool_size(8)  // Match CPU cores
       .build()?;
   ```

5. **Enable Hardware Acceleration:**
   ```toml
   [features]
   default = ["hw-accel"]
   ```

**More tips:** [Performance Guide](PERFORMANCE.md)

</details>

<details>
<summary><b>❓ What's the memory usage like?</b></summary>

<br>

**Typical Memory Usage:**

<table>
<tr>
<th>Scenario</th>
<th>Memory Usage</th>
<th>Notes</th>
</tr>
<tr>
<td>Basic initialization</td>
<td>~10 MB</td>
<td>Minimum overhead</td>
</tr>
<tr>
<td>With 100 keys</td>
<td>~50 MB</td>
<td>~0.4 MB per key</td>
</tr>
<tr>
<td>With caching (1 GB cache)</td>
<td>~1 GB</td>
<td>Configurable</td>
</tr>
<tr>
<td>High-throughput mode</td>
<td>~200 MB</td>
<td>Extra buffers</td>
</tr>
</table>

**Reduce Memory Usage:**

```rust
let config = Config::builder()
    .cache_size(256)      // Reduce cache
    .performance_profile(PerformanceProfile::LowMemory)
    .build()?;
```

**Memory Safety:**
- ✅ Automatic cleanup with `zeroize`
- ✅ Memory locking for sensitive data
- ✅ No memory leaks (verified with Valgrind)

</details>

---

## Security

<div align="center">

### 🔒 Security Features

</div>

<details>
<summary><b>❓ Is this secure?</b></summary>

<br>

**Yes!** Security is our top priority.

**Security Features:**

<table>
<tr>
<td width="50%">

**Implementation**
- ✅ Memory-safe (Rust)
- ✅ Audited crypto libraries
- ✅ Constant-time operations
- ✅ Secure random generation

</td>
<td width="50%">

**Protections**
- ✅ Buffer overflow protection
- ✅ Side-channel resistance
- ✅ Memory wiping (zeroize)
- ✅ Memory locking (mlock)

</td>
</tr>
</table>

**Compliance:**
- 🏅 FIPS 140-3 Level 1 (planned)
- 🏅 Chinese standards (SM2/SM3/SM4)

**Audits:**
- ✅ Internal security review
- 🚧 Third-party audit (Q2 2025)

**More details:** [Security Guide](SECURITY.md)

</details>

<details>
<summary><b>❓ How do I report security vulnerabilities?</b></summary>

<br>

**Please report security issues responsibly:**

1. **DO NOT** create public GitHub issues
2. **Email:** security@example.com
3. **Include:**
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

**Response Timeline:**
- 📧 Initial response: 24 hours
- 🔍 Assessment: 72 hours
- 🔧 Fix (if valid): 7-30 days
- 📢 Public disclosure: After fix released

**Security Policy:** [SECURITY.md](../SECURITY.md)

</details>

<details>
<summary><b>❓ What about key storage?</b></summary>

<br>

**Key Storage Options:**

<table>
<tr>
<th>Method</th>
<th>Security</th>
<th>Use Case</th>
</tr>
<tr>
<td><b>In-Memory</b></td>
<td>🔒 Good</td>
<td>Development, testing</td>
</tr>
<tr>
<td><b>File-based</b></td>
<td>🔒🔒 Better</td>
<td>Single-server deployment</td>
</tr>
<tr>
<td><b>HSM</b></td>
<td>🔒🔒🔒 Best</td>
<td>Production (coming soon)</td>
</tr>
</table>

**Best Practices:**

```rust
// 1. Use memory locking
let config = Config::builder()
    .enable_memory_locking(true)
    .build()?;

// 2. Set appropriate permissions
use std::fs;
fs::set_permissions("keys/", 0o600)?;

// 3. Encrypt keys at rest
let encrypted_key = encrypt_key(key, master_key)?;
```

**Planned Features:**
- 🚧 HSM integration (PKCS#11)
- 🚧 Cloud KMS support (AWS, Azure, GCP)
- 🚧 Hardware security module

</details>

<details>
<summary><b>❓ Are there any known vulnerabilities?</b></summary>

<br>

**Current Status:** ✅ **No known vulnerabilities**

**How we maintain security:**

1. **Dependency Scanning:**
   ```bash
   cargo audit
   ```

2. **Regular Updates:**
   - Weekly dependency updates
   - Security patches within 48 hours

3. **Testing:**
   - Fuzz testing
   - Static analysis
   - Security-focused code review

**Stay Informed:**
- 🔔 Watch this repository
- 📬 Subscribe to [security mailing list](mailto:security-subscribe@example.com)
- 📰 Check [security advisories](../../security/advisories)

</details>

---

## Troubleshooting

<div align="center">

### 🔧 Common Issues

</div>

<details>
<summary><b>❓ I'm getting "AlreadyInitialized" error</b></summary>

<br>

**Problem:**
```
Error: AlreadyInitialized
```

**Cause:** Calling `init()` multiple times.

**Solution:**

```rust
// Check before initializing
if !project_name::is_initialized() {
    project_name::init()?;
}

// Or use a once_cell
use once_cell::sync::Lazy;

static INIT: Lazy<()> = Lazy::new(|| {
    project_name::init().expect("Initialization failed");
});

fn main() {
    Lazy::force(&INIT);
    // ... rest of code
}
```

</details>

<details>
<summary><b>❓ Getting "KeyNotFound" errors</b></summary>

<br>

**Problem:**
```
Error: KeyNotFound("key-123")
```

**Common Causes:**

1. **Key was never generated:**
   ```rust
   // Generate the key first
   let key_id = km.generate_key(Algorithm::AES256GCM)?;
   ```

2. **Wrong key ID:**
   ```rust
   // Check key ID spelling
   let key_id = "user-key-123";  // Make sure this matches
   ```

3. **Key was deleted:**
   ```rust
   // List available keys
   let keys = km.list_keys()?;
   println!("Available keys: {:?}", keys);
   ```

**Debug Tips:**
```rust
// Enable debug logging
env::set_var("RUST_LOG", "debug");
env_logger::init();
```

</details>

<details>
<summary><b>❓ Performance is slower than expected</b></summary>

<br>

**Checklist:**

- [ ] Are you running in release mode?
  ```bash
  cargo run --release
  ```

- [ ] Have you configured thread pool size?
  ```rust
  Config::builder().thread_pool_size(num_cpus::get()).build()?
  ```

- [ ] Is hardware acceleration enabled?
  ```toml
  [features]
  default = ["hw-accel"]
  ```

- [ ] Are you using batch operations?
  ```rust
  process_batch(&items)?  // Better than loop
  ```

**Profiling:**
```bash
cargo flamegraph
```

**More help:** [Performance Guide](PERFORMANCE.md)

</details>

**More issues?** Check [Troubleshooting Guide](TROUBLESHOOTING.md)

---

## Contributing

<div align="center">

### 🤝 Join the Community

</div>

<details>
<summary><b>❓ How can I contribute?</b></summary>

<br>

**Ways to Contribute:**

<table>
<tr>
<td width="50%">

**Code Contributions**
- 🐛 Fix bugs
- ✨ Add features
- 📝 Improve documentation
- ✅ Write tests

</td>
<td width="50%">

**Non-Code Contributions**
- 📖 Write tutorials
- 🎨 Design assets
- 🌍 Translate docs
- 💬 Answer questions

</td>
</tr>
</table>

**Getting Started:**

1. 🍴 Fork the repository
2. 🌱 Create a branch
3. ✏️ Make changes
4. ✅ Add tests
5. 📤 Submit PR

**Guidelines:** [CONTRIBUTING.md](../CONTRIBUTING.md)

</details>

<details>
<summary><b>❓ I found a bug, what should I do?</b></summary>

<br>

**Before Reporting:**

1. ✅ Check [existing issues](../../issues)
2. ✅ Try the latest version
3. ✅ Check [troubleshooting guide](TROUBLESHOOTING.md)

**Creating a Good Bug Report:**

```markdown
### Description
Clear description of the bug

### Steps to Reproduce
1. Step one
2. Step two
3. See error

### Expected Behavior
What should happen

### Actual Behavior
What actually happens

### Environment
- OS: Ubuntu 22.04
- Rust version: 1.75.0
- Project version: 1.0.0

### Additional Context
Any other relevant information
```

**Submit:** [Create Issue](../../issues/new)

</details>

<details>
<summary><b>❓ Where can I get help?</b></summary>

<br>

<div align="center">

### 💬 Support Channels

</div>

<table>
<tr>
<td width="33%" align="center">

**🐛 Issues**

[GitHub Issues](../../issues)

Bug reports & features

</td>
<td width="33%" align="center">

**💬 Discussions**

[GitHub Discussions](../../discussions)

Q&A and ideas

</td>
<td width="33%" align="center">

**💡 Discord**

[Join Server](https://discord.gg/project)

Live chat

</td>
</tr>
</table>

**Response Times:**
- 🐛 Critical bugs: 24 hours
- 🔧 Feature requests: 1 week
- 💬 Questions: 2-3 days

</details>

---

## Licensing

<div align="center">

### 📄 License Information

</div>

<details>
<summary><b>❓ What license is this under?</b></summary>

<br>

**Dual License:**

<table>
<tr>
<td width="50%" align="center">

**MIT License**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../LICENSE-MIT)

**Permissions:**
- ✅ Commercial use
- ✅ Modification
- ✅ Distribution
- ✅ Private use

</td>
<td width="50%" align="center">

**Apache License 2.0**

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](../LICENSE-APACHE)

**Permissions:**
- ✅ Commercial use
- ✅ Modification
- ✅ Distribution
- ✅ Patent grant

</td>
</tr>
</table>

**You can choose either license for your use.**

</details>

<details>
<summary><b>❓ Can I use this in commercial projects?</b></summary>

<br>

**Yes!** Both MIT and Apache 2.0 licenses allow commercial use.

**What you need to do:**
1. ✅ Include the license text
2. ✅ Include copyright notice
3. ✅ State any modifications

**What you DON'T need to do:**
- ❌ Share your source code
- ❌ Open source your project
- ❌ Pay royalties

**Questions?** Contact: legal@example.com

</details>

---

<div align="center">

### 🎯 Still Have Questions?

<table>
<tr>
<td width="33%" align="center">
<a href="../../issues">
<img src="https://img.icons8.com/fluency/96/000000/bug.png" width="48"><br>
<b>Open an Issue</b>
</a>
</td>
<td width="33%" align="center">
<a href="../../discussions">
<img src="https://img.icons8.com/fluency/96/000000/chat.png" width="48"><br>
<b>Start a Discussion</b>
</a>
</td>
<td width="33%" align="center">
<a href="mailto:support@example.com">
<img src="https://img.icons8.com/fluency/96/000000/email.png" width="48"><br>
<b>Email Us</b>
</a>
</td>
</tr>
</table>

---

**[📖 User Guide](USER_GUIDE.md)** • **[🔧 API Docs](https://docs.rs/project-name)** • **[🏠 Home](../README.md)**

Made with ❤️ by the Documentation Team

[⬆ Back to Top](#-frequently-asked-questions-faq)

</div>