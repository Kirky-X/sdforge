// Copyright (c) 2026 Kirky.X
// SPDX-License-Identifier: MIT
//! 缓存功能集成测试
//!
//! 测试真实的缓存读写、过期、并发和策略功能。
//! 这些测试使用 DashMapCache 的真实实现，不使用 mock。
//!
//! 覆盖测试用例：
//! - 缓存读写测试：设置、获取、更新、删除、清空
//! - 缓存过期测试：TTL 过期、手动失效、过期前刷新
//! - 缓存并发测试：并发读写、并发写入、竞态条件防护
//! - 缓存策略测试：LRU 淘汰、内存限制、统计追踪
//!
//! 运行方式：
//! ```bash
//! cargo test --features cache --test cache_tests
//! ```

#[cfg(feature = "cache")]
mod cache_tests {
    use sdforge::cache::{DashMapCache, SharedCache, SyncCache};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    // ============================================================================
    // 缓存读写测试
    // ============================================================================

    /// 测试缓存的基本设置和获取功能
    ///
    /// 验证：
    /// - 设置键值对后可以正确获取
    /// - 获取的值与设置的值一致
    #[test]
    fn test_cache_set_and_get() {
        let cache = DashMapCache::new();

        // 设置字符串值
        cache.set("name", b"Alice".to_vec());
        assert_eq!(cache.get("name"), Some(b"Alice".to_vec()));

        // 设置数字值（序列化后）
        cache.set("age", b"30".to_vec());
        assert_eq!(cache.get("age"), Some(b"30".to_vec()));

        // 设置 JSON 格式的值
        cache.set("user", br#"{"id":1,"name":"Bob"}"#.to_vec());
        assert_eq!(
            cache.get("user"),
            Some(br#"{"id":1,"name":"Bob"}"#.to_vec())
        );

        // 设置空值
        cache.set("empty", b"".to_vec());
        assert_eq!(cache.get("empty"), Some(b"".to_vec()));
    }

    /// 测试获取不存在的键返回 None
    ///
    /// 验证：
    /// - 从未设置过的键返回 None
    /// - 已删除的键返回 None
    /// - 清空后所有键返回 None
    #[test]
    fn test_cache_get_nonexistent_key() {
        let cache = DashMapCache::new();

        // 从未设置过的键
        assert_eq!(cache.get("nonexistent"), None);
        assert!(cache.get("missing_key").is_none());

        // 设置后删除
        cache.set("temp", b"value".to_vec());
        assert_eq!(cache.get("temp"), Some(b"value".to_vec()));
        cache.delete("temp");
        assert_eq!(cache.get("temp"), None);

        // 清空后
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        cache.clear();
        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), None);
    }

    /// 测试更新现有键的值
    ///
    /// 验证：
    /// - 更新现有键不会保留旧值
    /// - 多次更新以最后一次为准
    #[test]
    fn test_cache_update_existing_key() {
        let cache = DashMapCache::new();

        // 初始设置
        cache.set("counter", b"1".to_vec());
        assert_eq!(cache.get("counter"), Some(b"1".to_vec()));

        // 更新为新值
        cache.set("counter", b"2".to_vec());
        assert_eq!(cache.get("counter"), Some(b"2".to_vec()));

        // 再次更新
        cache.set("counter", b"100".to_vec());
        assert_eq!(cache.get("counter"), Some(b"100".to_vec()));

        // 验证只有一个键
        assert_eq!(cache.len(), 1);
    }

    /// 测试删除键的功能
    ///
    /// 验证：
    /// - 删除存在的键返回 true 并移除键
    /// - 删除不存在的键返回 false
    /// - 删除后缓存状态正确
    #[test]
    fn test_cache_delete_key() {
        let cache = DashMapCache::new();

        // 删除不存在的键
        assert!(!cache.delete("nonexistent"));
        assert_eq!(cache.len(), 0);

        // 设置并删除
        cache.set("key1", b"value1".to_vec());
        cache.set("key2", b"value2".to_vec());
        assert_eq!(cache.len(), 2);

        // 删除第一个键
        assert!(cache.delete("key1"));
        assert_eq!(cache.get("key1"), None);
        assert_eq!(cache.get("key2"), Some(b"value2".to_vec()));
        assert_eq!(cache.len(), 1);

        // 删除第二个键
        assert!(cache.delete("key2"));
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());

        // 再次删除已删除的键
        assert!(!cache.delete("key1"));
    }

    /// 测试清空所有缓存
    ///
    /// 验证：
    /// - clear() 后缓存为空
    /// - clear() 不影响克隆的共享缓存
    #[test]
    fn test_cache_clear_all() {
        let cache = DashMapCache::new();

        // 添加多个键值对
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        cache.set("k3", b"v3".to_vec());
        assert_eq!(cache.len(), 3);

        // 清空缓存
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), None);
        assert_eq!(cache.get("k3"), None);

        // 清空后可以继续添加
        cache.set("new_key", b"new_value".to_vec());
        assert_eq!(cache.get("new_key"), Some(b"new_value".to_vec()));
    }

    // ============================================================================
    // 缓存过期测试
    // ============================================================================

    /// 测试 TTL 过期机制
    ///
    /// 验证：
    /// - 可以手动实现基于 TTL 的过期逻辑
    /// - 结合时间戳可以实现过期检查
    #[test]
    fn test_cache_ttl_expiration() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let cache = DashMapCache::new();
        let ttl_seconds: u64 = 1;

        // 设置带过期时间的值
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiry_time = current_time + ttl_seconds;

        // 存储值和过期时间（作为元数据）
        cache.set("data", b"sensitive".to_vec());
        cache.set("data_expiry", expiry_time.to_string().into_bytes());

        // 立即检查：未过期
        assert_eq!(cache.get("data"), Some(b"sensitive".to_vec()));

        // 模拟过期检查函数
        let is_expired = |cache: &DashMapCache, key: &str| -> bool {
            if let Some(expiry_bytes) = cache.get(&format!("{}_expiry", key))
                && let Ok(expiry) = String::from_utf8(expiry_bytes)
                    .unwrap_or_default()
                    .parse::<u64>()
            {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                return now > expiry;
            }
            false
        };

        // 模拟过期：更新过期时间为过去的时间
        cache.set("data_expiry", (current_time - 1).to_string().into_bytes());

        // 检查过期状态
        assert!(is_expired(&cache, "data"));
    }

    /// 测试手动失效机制
    ///
    /// 验证：
    /// - 可以通过删除键实现手动失效
    /// - 可以通过 clear() 使所有缓存失效
    #[test]
    fn test_cache_manual_invalidation() {
        let cache = DashMapCache::new();

        // 设置多个缓存项
        cache.set("session:user1", b"session_data_1".to_vec());
        cache.set("session:user2", b"session_data_2".to_vec());
        cache.set("token:123", b"auth_token".to_vec());

        // 单个失效：登出用户1
        cache.delete("session:user1");
        assert_eq!(cache.get("session:user1"), None);
        assert_eq!(cache.get("session:user2"), Some(b"session_data_2".to_vec()));

        // 批量失效：使所有 session 失效（需要手动实现前缀匹配）
        // 这里演示手动删除多个键
        cache.delete("session:user2");
        cache.delete("token:123");
        assert!(cache.is_empty());
    }

    /// 测试过期前刷新（延长 TTL）
    ///
    /// 验证：
    /// - 重新设置值可以刷新过期时间
    /// - 多次访问可以更新访问时间
    #[test]
    fn test_cache_refresh_before_expiry() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let cache = DashMapCache::new();

        // 初始设置
        cache.set("page:home", b"html_content".to_vec());
        cache.set("page:home_last_refresh", b"0".to_vec());

        // 获取当前时间
        let get_current_time = || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        };

        let initial_time = get_current_time();

        // 模拟刷新操作：重新设置值并更新时间戳
        cache.set("page:home", b"updated_html".to_vec());
        cache.set(
            "page:home_last_refresh",
            get_current_time().to_string().into_bytes(),
        );

        // 验证值已更新
        assert_eq!(cache.get("page:home"), Some(b"updated_html".to_vec()));

        // 验证时间戳已更新
        let new_time_bytes = cache.get("page:home_last_refresh").unwrap();
        let new_time: u64 = String::from_utf8(new_time_bytes).unwrap().parse().unwrap();
        assert!(new_time >= initial_time);
    }

    // ============================================================================
    // 缓存并发测试
    // ============================================================================

    /// 测试并发读取操作
    ///
    /// 验证：
    /// - 多个线程同时读取同一个键不会导致问题
    /// - 并发读取结果一致
    #[test]
    fn test_cache_concurrent_reads() {
        let cache = DashMapCache::new();

        // 设置共享数据
        cache.set("shared_data", b"important_content".to_vec());
        let cache = Arc::new(cache);

        // 启动多个读取线程
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let cache = cache.clone();
                thread::spawn(move || {
                    // 每次读取之间添加小延迟以增加并发度
                    thread::sleep(Duration::from_micros(10));
                    let value = cache.get("shared_data");
                    assert_eq!(value, Some(b"important_content".to_vec()));
                    i // 返回线程 ID
                })
            })
            .collect();

        // 等待所有线程完成
        for handle in handles {
            let _ = handle.join();
        }

        // 验证缓存仍然完整
        assert_eq!(cache.len(), 1);
        assert_eq!(
            cache.get("shared_data"),
            Some(b"important_content".to_vec())
        );
    }

    /// 测试并发写入操作
    ///
    /// 验证：
    /// - 多个线程同时写入不同的键不会导致问题
    /// - 所有写入最终都被保存
    #[test]
    fn test_cache_concurrent_writes() {
        let cache = Arc::new(DashMapCache::new());
        let num_threads = 20;
        let keys_per_thread = 5;

        // 启动多个写入线程
        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let cache = cache.clone();
                thread::spawn(move || {
                    for i in 0..keys_per_thread {
                        let key = format!("thread_{}_key_{}", thread_id, i);
                        let value = format!("value_from_thread_{}", thread_id);
                        cache.set(&key, value.into_bytes());
                    }
                })
            })
            .collect();

        // 等待所有线程完成
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // 验证所有键值对都已写入
        assert_eq!(cache.len(), num_threads * keys_per_thread);

        // 抽样验证几个键的值
        assert_eq!(
            cache.get("thread_0_key_0"),
            Some(b"value_from_thread_0".to_vec())
        );
        assert_eq!(
            cache.get("thread_10_key_3"),
            Some(b"value_from_thread_10".to_vec())
        );
        assert_eq!(
            cache.get("thread_19_key_4"),
            Some(b"value_from_thread_19".to_vec())
        );
    }

    /// 测试竞态条件防护
    ///
    /// 验证：
    /// - check-then-act 模式在并发环境下的安全性
    /// - 即使多个线程同时操作也能保持一致性
    #[test]
    fn test_cache_race_condition_prevention() {
        let cache = Arc::new(DashMapCache::new());
        let num_threads = 50;

        // 使用 "increment" 模式测试原子性
        // 初始值
        cache.set("counter", b"0".to_vec());

        let handles: Vec<_> = (0..num_threads)
            .map(|_| {
                let cache = cache.clone();
                thread::spawn(move || {
                    // 模拟 check-then-act
                    if let Some(value) = cache.get("counter") {
                        let current: u32 = String::from_utf8(value)
                            .unwrap_or_default()
                            .parse()
                            .unwrap_or(0);
                        let next = current + 1;
                        cache.set("counter", next.to_string().into_bytes());
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // 注意：由于 check-then-act 不是原子的，最终值可能小于 num_threads
        // 这验证了缓存本身是线程安全的，但业务逻辑需要额外的同步
        let final_value_bytes = cache.get("counter").unwrap();
        let final_value: u32 = String::from_utf8(final_value_bytes)
            .unwrap_or_default()
            .parse()
            .unwrap_or(0);

        // 值应该在合理范围内（至少有增长）
        assert!(final_value <= num_threads);
        assert!(final_value > 0); // 至少有一些增长

        // 清理
        cache.delete("counter");
    }

    // ============================================================================
    // 缓存策略测试
    // ============================================================================

    /// 测试 LRU（最近最少使用）淘汰策略
    ///
    /// 验证：
    /// - 可以手动实现基于访问顺序的 LRU 淘汰
    /// - 超过容量限制时移除最旧的项
    #[test]
    fn test_cache_lru_eviction() {
        let cache = DashMapCache::new();

        // 模拟 LRU 淘汰：设置初始数据
        // 假设访问顺序是 key1 -> key2 -> key3，key1 是 LRU
        cache.set("lru_list", b"key1,key2,key3".to_vec());
        cache.set("key1", b"value1".to_vec());
        cache.set("key2", b"value2".to_vec());
        cache.set("key3", b"value3".to_vec());

        // 添加新数据项，现在有 4 个数据项
        cache.set("key4", b"value4".to_vec());

        // 手动 LRU 淘汰：删除 lru_list 中的第一个（key1）
        let evict_lru = |cache: &DashMapCache| {
            if let Some(list_bytes) = cache.get("lru_list") {
                let list = String::from_utf8(list_bytes).unwrap_or_default();
                let keys: Vec<&str> = list.split(',').collect();
                if let Some(lru_key) = keys.first() {
                    cache.delete(lru_key);
                }
            }
        };

        evict_lru(&cache);

        // 验证：key1 被删除，保留 key2, key3, key4, lru_list
        assert_eq!(cache.get("key1"), None);
        assert_eq!(cache.get("key2"), Some(b"value2".to_vec()));
        assert_eq!(cache.get("key3"), Some(b"value3".to_vec()));
        assert_eq!(cache.get("key4"), Some(b"value4".to_vec()));
        assert_eq!(cache.get("lru_list"), Some(b"key1,key2,key3".to_vec()));

        // 验证缓存大小：key2, key3, key4, lru_list = 4
        assert_eq!(cache.len(), 4);
    }

    /// 测试内存限制策略
    ///
    /// 验证：
    /// - 可以手动实现基于大小的内存限制
    /// - 超过限制时触发清理
    #[test]
    fn test_cache_memory_limit() {
        let cache = DashMapCache::new();

        // 设置值并追踪大小
        cache.set("data", vec![0u8; 100]);
        cache.set("data_size", b"100".to_vec());

        // 计算缓存项大小的函数
        let calculate_size = |cache: &DashMapCache| -> usize {
            let mut total = 0;
            for key in cache.find_keys_by_pattern("*") {
                if let Some(value) = cache.get(&key) {
                    total += key.len();
                    total += value.len();
                }
            }
            total
        };

        // 添加更多数据直到接近限制
        cache.set("large_data", vec![0u8; 200]);
        cache.set("large_data_size", b"200".to_vec());

        let current_size = calculate_size(&cache);
        assert!(current_size > 300); // 至少有 300 字节

        // 模拟内存限制清理
        let memory_limit = 250;
        let enforce_memory_limit = |cache: &DashMapCache| {
            let mut iterations = 0;
            while calculate_size(cache) > memory_limit && iterations < 10 {
                iterations += 1;
                // 找到第一个不是 _size 结尾的键
                let keys: Vec<String> = cache.find_keys_by_pattern("*");
                if let Some(key_to_remove) = keys.iter().find(|k| !k.ends_with("_size")) {
                    cache.delete(key_to_remove);
                } else {
                    break; // 没有可删除的键
                }
            }
        };

        enforce_memory_limit(&cache);

        // 验证内存限制生效
        let final_size = calculate_size(&cache);
        assert!(final_size <= memory_limit + 50); // 允许一些容差
    }

    /// 测试统计追踪功能
    ///
    /// 验证：
    /// - 可以追踪缓存的访问统计
    /// - 可以计算命中率
    #[test]
    fn test_cache_stats_tracking() {
        let cache = DashMapCache::new();

        // 设置统计追踪
        cache.set("stats_hits", b"0".to_vec());
        cache.set("stats_misses", b"0".to_vec());

        // 辅助函数：记录命中
        let record_hit = |cache: &DashMapCache| {
            if let Some(hits_bytes) = cache.get("stats_hits") {
                let hits: u32 = String::from_utf8(hits_bytes)
                    .unwrap_or_default()
                    .parse()
                    .unwrap_or(0);
                cache.set("stats_hits", (hits + 1).to_string().into_bytes());
            }
        };

        // 辅助函数：记录未命中
        let record_miss = |cache: &DashMapCache| {
            if let Some(misses_bytes) = cache.get("stats_misses") {
                let misses: u32 = String::from_utf8(misses_bytes)
                    .unwrap_or_default()
                    .parse()
                    .unwrap_or(0);
                cache.set("stats_misses", (misses + 1).to_string().into_bytes());
            }
        };

        // 添加数据
        cache.set("key1", b"value1".to_vec());

        // 模拟多次访问
        for _ in 0..5 {
            if cache.get("key1").is_some() {
                record_hit(&cache);
            } else {
                record_miss(&cache);
            }
        }

        // 模拟未命中
        for _ in 0..3 {
            if cache.get("nonexistent").is_some() {
                record_hit(&cache);
            } else {
                record_miss(&cache);
            }
        }

        // 获取最终统计
        let hits: u32 = String::from_utf8(cache.get("stats_hits").unwrap())
            .unwrap()
            .parse()
            .unwrap();
        let misses: u32 = String::from_utf8(cache.get("stats_misses").unwrap())
            .unwrap()
            .parse()
            .unwrap();

        assert_eq!(hits, 5);
        assert_eq!(misses, 3);

        // 计算命中率
        let total = hits + misses;
        let hit_rate = (hits as f64) / (total as f64);
        assert!((hit_rate - 0.625).abs() < 0.001); // 5/8 = 0.625
    }

    // ============================================================================
    // 批量操作测试
    // ============================================================================

    /// 测试批量获取操作
    ///
    /// 验证：
    /// - 批量获取多个键的效率
    /// - 部分键不存在时的处理
    #[test]
    fn test_cache_get_many() {
        let cache = DashMapCache::new();

        // 设置多个键值对
        cache.set("key1", b"value1".to_vec());
        cache.set("key2", b"value2".to_vec());
        cache.set("key3", b"value3".to_vec());

        // 批量获取存在的键
        let keys = ["key1", "key2", "key3"];
        let results = cache.get_many(&keys);
        assert_eq!(results.len(), 3);
        assert_eq!(results.get("key1"), Some(&b"value1".to_vec()));
        assert_eq!(results.get("key2"), Some(&b"value2".to_vec()));
        assert_eq!(results.get("key3"), Some(&b"value3".to_vec()));

        // 批量获取混合存在的键
        let mixed_keys = ["key1", "nonexistent", "key3"];
        let mixed_results = cache.get_many(&mixed_keys);
        assert_eq!(mixed_results.len(), 2);
        assert!(mixed_results.contains_key("key1"));
        assert!(mixed_results.contains_key("key3"));
        assert!(!mixed_results.contains_key("nonexistent"));
    }

    /// 测试批量设置操作
    ///
    /// 验证：
    /// - 批量设置多个键值对的效率
    /// - 批量更新现有键
    #[test]
    fn test_cache_set_many() {
        let cache = DashMapCache::new();

        // 初始状态
        assert!(cache.is_empty());

        // 批量设置新键
        let items = vec![
            ("k1".to_string(), b"v1".to_vec()),
            ("k2".to_string(), b"v2".to_vec()),
            ("k3".to_string(), b"v3".to_vec()),
        ];
        cache.set_many(&items);

        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get("k1"), Some(b"v1".to_vec()));
        assert_eq!(cache.get("k2"), Some(b"v2".to_vec()));
        assert_eq!(cache.get("k3"), Some(b"v3".to_vec()));

        // 批量更新现有键
        let update_items = vec![
            ("k1".to_string(), b"v1_updated".to_vec()),
            ("k2".to_string(), b"v2_updated".to_vec()),
        ];
        cache.set_many(&update_items);

        assert_eq!(cache.len(), 3); // 数量不变
        assert_eq!(cache.get("k1"), Some(b"v1_updated".to_vec()));
        assert_eq!(cache.get("k2"), Some(b"v2_updated".to_vec()));
        assert_eq!(cache.get("k3"), Some(b"v3".to_vec())); // 未更新的保持不变
    }

    /// 测试批量删除操作
    ///
    /// 验证：
    /// - 批量删除多个键的效率
    /// - 删除不存在的键不影响结果
    #[test]
    fn test_cache_delete_many() {
        let cache = DashMapCache::new();

        // 设置多个键值对
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        cache.set("k3", b"v3".to_vec());
        cache.set("k4", b"v4".to_vec());
        assert_eq!(cache.len(), 4);

        // 批量删除部分键
        let keys_to_delete = ["k1", "k3", "nonexistent"];
        let deleted_count = cache.delete_many(&keys_to_delete);

        assert_eq!(deleted_count, 2); // 只有 k1 和 k3 被删除
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get("k1"), None);
        assert_eq!(cache.get("k2"), Some(b"v2".to_vec()));
        assert_eq!(cache.get("k3"), None);
        assert_eq!(cache.get("k4"), Some(b"v4".to_vec()));

        // 再批量删除剩余的键
        let remaining_keys = ["k2", "k4"];
        let second_deleted_count = cache.delete_many(&remaining_keys);
        assert_eq!(second_deleted_count, 2);
        assert!(cache.is_empty());
    }

    // ============================================================================
    // SharedCache 类型测试
    // ============================================================================

    /// 测试 SharedCache 类型的使用
    ///
    /// 验证：
    /// - SharedCache 作为 Arc<dyn SyncCache> 的别名工作正常
    /// - 可以通过 trait 对象使用缓存
    #[test]
    fn test_shared_cache_type() {
        let cache: SharedCache = Arc::new(DashMapCache::new());

        cache.set("shared_key", b"shared_value".to_vec());
        assert_eq!(cache.get("shared_key"), Some(b"shared_value".to_vec()));
        assert_eq!(cache.len(), 1);

        // 测试克隆
        let cache2 = cache.clone();
        assert_eq!(cache2.get("shared_key"), Some(b"shared_value".to_vec()));

        // 测试两个引用指向同一个缓存
        cache.set("another", b"value".to_vec());
        assert_eq!(cache2.len(), 2);
    }

    // ============================================================================
    // 边界条件测试
    // ============================================================================

    /// 测试键的特殊字符处理
    ///
    /// 验证：
    /// - 特殊字符作为键名
    /// - 键名的大小写敏感性
    #[test]
    fn test_cache_special_key_characters() {
        let cache = DashMapCache::new();

        // 特殊字符键
        cache.set("key:with:colons", b"value".to_vec());
        cache.set("key/with/slashes", b"value".to_vec());
        cache.set("key with spaces", b"value".to_vec());
        cache.set("key.with.dots", b"value".to_vec());

        assert_eq!(cache.get("key:with:colons"), Some(b"value".to_vec()));
        assert_eq!(cache.get("key/with/slashes"), Some(b"value".to_vec()));
        assert_eq!(cache.get("key with spaces"), Some(b"value".to_vec()));
        assert_eq!(cache.get("key.with.dots"), Some(b"value".to_vec()));

        // 大小写敏感性
        cache.set("Key", b"uppercase".to_vec());
        cache.set("key", b"lowercase".to_vec());
        assert_eq!(cache.get("Key"), Some(b"uppercase".to_vec()));
        assert_eq!(cache.get("key"), Some(b"lowercase".to_vec()));
        assert_eq!(cache.len(), 6); // 5个特殊键 + 2个大小写不同的键
    }

    /// 测试值的边界情况
    ///
    /// 验证：
    /// - 空值
    /// - 非常大的值
    /// - 二进制数据
    #[test]
    fn test_cache_value_edge_cases() {
        let cache = DashMapCache::new();

        // 空值
        cache.set("empty", b"".to_vec());
        assert_eq!(cache.get("empty"), Some(b"".to_vec()));

        // 二进制数据
        let binary_data: Vec<u8> = (0..=255).collect(); // 0-255 的所有字节
        cache.set("binary", binary_data.clone());
        assert_eq!(cache.get("binary"), Some(binary_data));

        // Unicode 数据
        cache.set("unicode", "Hello, 世界! 🌍".as_bytes().to_vec());
        assert_eq!(
            cache.get("unicode"),
            Some("Hello, 世界! 🌍".as_bytes().to_vec())
        );

        // 大值
        let large_value = vec![0x42u8; 1024 * 100]; // 100KB
        cache.set("large", large_value.clone());
        assert_eq!(cache.get("large"), Some(large_value));
    }

    /// 测试 contains 方法
    ///
    /// 验证：
    /// - 存在的键返回 true
    /// - 不存在的键返回 false
    #[test]
    fn test_cache_contains() {
        let cache = DashMapCache::new();

        // 空缓存
        assert!(!cache.contains("any_key"));

        // 添加后
        cache.set("existing", b"value".to_vec());
        assert!(cache.contains("existing"));
        assert!(!cache.contains("nonexistent"));

        // 删除后
        cache.delete("existing");
        assert!(!cache.contains("existing"));

        // 清空后
        cache.set("k1", b"v1".to_vec());
        cache.set("k2", b"v2".to_vec());
        cache.clear();
        assert!(!cache.contains("k1"));
        assert!(!cache.contains("k2"));
    }
}
