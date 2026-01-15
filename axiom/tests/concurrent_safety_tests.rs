//! Concurrent Safety Tests
//!
//! Tests for thread safety and concurrent access patterns.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Barrier;

#[cfg(test)]
mod concurrent_safety_tests {
    use super::*;

    /// Test: Lock-free data structure behavior
    #[tokio::test]
    async fn test_lock_free_operations() {
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        for _ in 0..10 {
            let counter = counter.clone();
            let barrier = barrier.clone();
            let handle = tokio::spawn(async move {
                barrier.wait().await;
                for _ in 0..1000 {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(
            counter.load(Ordering::SeqCst),
            10000,
            "All increments should be counted"
        );
    }

    /// Test: Concurrent service response building
    #[tokio::test]
    async fn test_service_response_concurrent_build() {
        use axiom::core::ServiceResponse;

        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        for _i in 0..10 {
            let barrier = barrier.clone();
            let handle = tokio::spawn(async move {
                barrier.wait().await;
                for _ in 0..100 {
                    let _response = ServiceResponse::success("test_data");
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Test: Concurrent ApiError creation
    #[tokio::test]
    async fn test_api_error_concurrent_creation() {
        use axiom::prelude::ApiError;

        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        for i in 0..10 {
            let barrier = barrier.clone();
            let handle = tokio::spawn(async move {
                barrier.wait().await;
                for j in 0..100 {
                    let _error = ApiError::NotFound {
                        resource: format!("resource_{}", i),
                        resource_id: Some(format!("id_{}", j)),
                    };
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    /// Stress test: High load on atomic operations
    #[tokio::test]
    async fn test_atomic_stress() {
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(100));
        let mut handles = vec![];

        for _ in 0..100 {
            let counter = counter.clone();
            let barrier = barrier.clone();
            let handle = tokio::spawn(async move {
                barrier.wait().await;
                for _ in 0..100 {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10000);
    }

    /// Stress test: Multiple concurrent ServiceResponse builds
    #[tokio::test]
    async fn test_response_stress() {
        use axiom::core::ServiceResponse;

        let barrier = Arc::new(Barrier::new(50));
        let mut handles = vec![];

        for _ in 0..50 {
            let barrier = barrier.clone();
            let handle = tokio::spawn(async move {
                barrier.wait().await;
                for _ in 0..200 {
                    let _response = ServiceResponse::success("stress_test_data");
                    let _response = ServiceResponse::<String>::error(
                        axiom::core::ServiceError::new("ERROR", "error_message", 500),
                    );
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }
}

#[cfg(test)]
mod thread_safety_tests {
    use super::*;

    /// Test: Arc cloning thread safety
    #[tokio::test]
    async fn test_arc_cloning_thread_safety() {
        let value = Arc::new(42);
        let barrier = Arc::new(Barrier::new(10));
        let mut handles = vec![];

        for _ in 0..10 {
            let value = value.clone();
            let barrier = barrier.clone();
            let handle = tokio::spawn(async move {
                barrier.wait().await;
                assert_eq!(*value, 42);
                *value
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Test: Send/Sync bounds verification
    #[test]
    fn test_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<AtomicUsize>();
        assert_send_sync::<Barrier>();

        // These should also be Send + Sync
        assert_send_sync::<axiom::core::ServiceResponse<String>>();
        assert_send_sync::<axiom::prelude::ApiError>();
    }
}
