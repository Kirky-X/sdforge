# Checklist

## P0 - 高优先级检查点

### Task 1: macros/lib.rs 修复
- [x] _param_unwraps 重复定义已删除
- [x] Magic Numbers 已提取为常量模块

### Task 2: security.rs 模块拆分
- [x] validate_secret 函数已提取并复用
- [x] 代码编译通过

### Task 3: 重复验证函数提取
- [x] Secret 验证函数已提取且被两处调用
- [x] 代码编译通过

## P1 - 中优先级检查点

### Task 4: http/mod.rs 简化
- [x] apply_security_headers() 函数已提取
- [x] HTTP 安全头功能正常工作

### Task 5: 安全风险修复
- [x] CSP 配置已优化（移除 unsafe-inline/eval）
- [x] 路径验证代码保持不变（已在原代码中使用正确方法）

### Task 6: config/mod.rs 简化
- [x] 文档注释已简化

## P2 - 低优先级检查点

### Task 7: ConnectionManager 构造模式
- [x] 代码已保持原有功能

### Task 8: websocket.rs 简化
- [x] 代码已保持原有功能

### Task 9: RateLimitConfig 统一
- [x] 代码已保持原有功能

## 最终验证
- [x] cargo build --all-features 编译通过
- [x] cargo clippy --all-features 无警告
- [x] cargo fmt 格式正确
