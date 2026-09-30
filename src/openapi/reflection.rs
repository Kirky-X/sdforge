// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT
//! 返回类型 Schema 反射（两 trait 方法解析探针）。
//!
//! `#[forge]` 宏为每个有返回类型的端点发射一个内联具名函数，函数体内
//! 调用 [`SchemaProbe::<T>::new().probe()`]——**方法解析发生在宏发射点的
//! 具体返回类型上**。泛型包装层不可行：泛型函数体的方法解析只用形参
//! 约束求解，`T: 'static` 无法证明 `T: JsonSchema`，精确分支恒被拒、
//! 退化为恒兜底，故不存在 `fn reflect<T>()` 形态的入口。
//!
//! 两个 trait 候选驻留在方法解析的不同步骤，从机制上排除多候选歧义：
//!
//! 1. 按值探测 [`PreciseSchema`]（步骤 (0,0)，`self` 按值）：`T` 派生
//!    `JsonSchema` 时命中，产出完整 JSON Schema 文本；
//! 2. 未命中时 autoref 探测 [`FallbackSchema`]（步骤 (0,1)，`&self`），
//!    静默返回 `None`，由 `generate_openapi_spec` 降级到粗粒度映射。
//!
//! `schemars` feature 关闭时 [`PreciseSchema`] 无任何实现，探针恒走兜底
//! 分支——发射点 token 两态同构，无需条件编译。

/// 方法解析探针。仅作方法解析的接收者载体，无运行时数据。
pub struct SchemaProbe<T>(core::marker::PhantomData<T>);

impl<T> SchemaProbe<T> {
    /// 创建探针（无运行时开销）。
    #[must_use]
    pub const fn new() -> Self {
        Self(core::marker::PhantomData)
    }
}

impl<T> Default for SchemaProbe<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// 精确分支：`T: JsonSchema` 时方法解析按值优先命中本实现。
///
/// `probe` 以 `self` 按值接收——驻留在解析步骤 (0,0)，先于兜底分支的
/// autoref 步骤，两者不共存于同一步骤。
pub trait PreciseSchema {
    /// 返回 `T` 的 JSON Schema 序列化文本。
    fn probe(self) -> Option<String>;
}

#[cfg(feature = "schemars")]
impl<T: schemars::JsonSchema> PreciseSchema for SchemaProbe<T> {
    fn probe(self) -> Option<String> {
        serde_json::to_string(&schemars::schema_for!(T)).ok()
    }
}

/// 兜底分支：任意 `T` 在精确分支不可解时经 autoref（`&SchemaProbe<T>`）
/// 命中，静默降级。
pub trait FallbackSchema {
    /// 恒 `None`：未派生类型无可反射的 schema。
    fn probe(&self) -> Option<String> {
        None
    }
}

impl<T> FallbackSchema for SchemaProbe<T> {}

#[cfg(test)]
mod tests {
    // 两态共享的候选导入：schemars 开启态精确分支不命中时 PreciseSchema 未用
    #[allow(unused_imports)]
    use super::{FallbackSchema, PreciseSchema, SchemaProbe};

    /// 兜底分支对未派生类型恒 `None`。schemars 开启态 std 类型内建
    /// `JsonSchema` 会命中精确分支，兜底只能以本地未派生类型验证；
    /// 关闭态任意类型（含 std）恒兜底。
    #[test]
    fn fallback_probe_is_none_for_plain_types() {
        #[allow(dead_code)]
        struct Plain {
            inner: u8,
        }
        assert!(SchemaProbe::<Plain>::new().probe().is_none());

        #[cfg(not(feature = "schemars"))]
        {
            assert!(SchemaProbe::<String>::new().probe().is_none());
            assert!(SchemaProbe::<Vec<u8>>::new().probe().is_none());
        }
    }
}
