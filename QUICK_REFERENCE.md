# SDForge 代码审查快速参考卡

> 一页纸总结 - 贴在显示器旁边随时提醒！

---

## 🔴 立即修复（今天完成）

### ✅ Cargo.toml 路径问题
```toml
# ❌ 错误
confers = { path = "/home/dev/projects/confers" }

# ✅ 正确
confers = { path = "../confers" }
# 或
confers = { git = "https://github.com/Kirky-X/confers" }
```

### ✅ JWT Secret 验证
```rust
// ❌ 缺少验证
Self::try_new(secret).expect("Failed");

// ✅ 添加验证
if secret.len() < 32 {
    return Err(AuthError::WeakSecret("Must be 32+ bytes".into()));
}
```

---

## 🟡 本周开始（High Priority）

### 1. 全局状态 → 上下文对象
```rust
// ❌ 旧模式（全局静态）
static ROUTES: OnceLock<Mutex<Vec<...>>> = OnceLock::new();

// ✅ 新模式（显式注入）
let ctx = SdForgeContext::new();
ctx.register_http(route);
app.with_state(Arc::new(ctx))
```

### 2. 滑动窗口限流
```rust
// ❌ 固定窗口（可绕过）
if current_window != stored_window { count = 1; }

// ✅ 滑动窗口（加权平均）
let weight = 1.0 - (elapsed / window_size);
let weighted = prev_count * weight + current_count;
```

### 3. 统一错误类型
```rust
// ❌ 混用多种错误
Result<T, ApiError>
Result<T, anyhow::Error>
Option<T>

// ✅ 统一错误
type Result<T> = std::result::Result<T, SdForgeError>;
```

---

## 🟢 本月完成（Medium Priority）

### 1. 消除 Builder 重复
```rust
// ❌ 手写重复代码
pub struct MyBuilder { field: Option<T> }
impl MyBuilder {
    pub fn new() -> Self { Self { field: None } }
    pub fn field(mut self, v: T) -> Self { self.field = Some(v); self }
}

// ✅ 使用宏生成
builder_pattern! {
    pub struct MyBuilder { field: Option<T> }
    target: MyStruct,
}
```

### 2. Regex 缓存优化
```rust
// ❌ 克隆开销大
static CACHE: Lazy<DashMap<String, Regex>> = ...;
cached.clone()  // 昂贵

// ✅ 使用 Arc
static CACHE: Lazy<DashMap<String, Arc<Regex>>> = ...;
Arc::clone(&cached)  // 廉价
```

### 3. HTTP 方法枚举化
```rust
// ❌ 字符串匹配
match method.as_str() {
    "get" => ...,
    "post" => ...,
}

// ✅ 类型安全
match HttpMethod::from_str(method) {
    Some(HttpMethod::Get) => ...,
    Some(HttpMethod::Post) => ...,
}
```

---

## 📊 质量检查清单

### 提交前自检
- [ ] 没有硬编码路径
- [ ] 密钥长度 >= 32 字节
- [ ] 所有输入都有验证
- [ ] 错误处理一致
- [ ] 没有 TODO 标记未处理
- [ ] 测试通过且覆盖关键路径

### 安全红线
- 🔴 绝不接受弱密钥
- 🔴 绝不泄露敏感信息
- 🔴 绝不跳过输入验证
- 🔴 绝不使用 expect 处理用户输入

### 性能提示
- 💡 热点代码使用 Arc 避免克隆
- 💡 高频操作使用缓存
- 💡 大对象使用对象池
- 💡 并发场景考虑锁竞争

---

## 🎯 优先级矩阵

```
        重要性
         高 |  ① JWT 验证    ② 全局状态
            |  ③ 限流算法   ④ 错误统一
            |
         低 |  ⑤ 清理死代码  ⑥ 文档更新
            |________________________
              紧急      不紧急
                 时间
```

**策略**: 先做左上角（重要且紧急），再做右上角

---

## 📈 目标指标

| 指标 | 当前 | 目标 | 期限 |
|------|------|------|------|
| 安全评分 | 75 | 95 | 4 月底 |
| 架构评分 | 80 | 90 | 5 月底 |
| 性能评分 | 85 | 90 | 6 月底 |
| 测试覆盖 | ? | >80% | 7 月底 |

---

## 🔗 有用链接

- 📖 [完整审查报告](CODE_REVIEW_REPORT.md)
- 🛠️ [实施指南](OPTIMIZATION_GUIDE.md)
- 📋 [TODO 追踪](TODO_TRACKER.md)
- 📊 [总结文档](CODE_REVIEW_SUMMARY.md)

---

## 💬 常用命令

```bash
# 检查编译
cargo check --all-features

# 运行测试
cargo test --all-features

# 格式化代码
cargo fmt

# Lint 检查
cargo clippy --all-features -- -D warnings

# 查看依赖
cargo tree

# 安全检查
cargo audit
```

---

## 🚨 常见陷阱

### ❌ 不要这样做
```rust
// 1. 硬编码
path = "/home/user/project/..."  // 绝对禁止！

// 2. 弱密钥
jwt_secret = "123456"  // 太短！

// 3. 忽略错误
.do_something().unwrap()  // 用户输入不能用 unwrap！

// 4. 全局可变状态
static mut GLOBAL: ...  // 绝对禁止！
```

### ✅ 要这样做
```rust
// 1. 相对路径或配置
path = "../dependency"

// 2. 强密钥
generate_secure_jwt_secret()  // 使用工具生成

// 3. 优雅处理错误
.do_something()?  // 或使用 match

// 4. 显式依赖注入
State(ctx): State<Arc<SdForgeContext>>
```

---

## 🎓 学习资源

- **Rust 安全编程**: https://doc.rust-lang.org/nomicon/
- **OWASP Top 10**: https://owasp.org/www-project-top-ten/
- **十二要素应用**: https://12factor.net/
- **Clean Architecture**: Robert C. Martin

---

## 📞 求助渠道

遇到问题？

1. 查看文档
2. 搜索 Issue
3. 提问 Slack
4. 创建 Issue

---

**记住**: 每次提交都让代码更好一点！🚀

*最后更新：2026-03-31*
