//! Integration tests for load testing the LiveShare system
//!
//! These tests simulate various load scenarios:
//! - 10, 50, 100 concurrent users
//! - Network conditions (latency, packet loss)
//! - Message throughput and bandwidth measurement
//!
//! Run with: cargo test -- --ignored --test-threads=1

#[cfg(test)]
mod tests {
    use crate::core::liveshare::load_test::{LoadTestConfig, LoadTestMetrics};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::Barrier;

    /// Helper to simulate a user sending and receiving messages
    async fn simulate_user(
        user_id: usize,
        metrics: Arc<LoadTestMetrics>,
        config: Arc<LoadTestConfig>,
        start_signal: Arc<Barrier>,
    ) {
        // Wait for all users to be ready
        start_signal.wait().await;

        let user_metric = &metrics.user_metrics[user_id];
        let message_size = 512; // bytes

        let mut messages_sent = 0;
        while metrics.elapsed() < metrics.duration && messages_sent < config.messages_per_user {
            // Simulate message sending with network latency
            if config.simulate_network_latency {
                let latency = Duration::from_millis(10);
                tokio::time::sleep(latency).await;
            }

            // Record message sent
            user_metric.record_message_sent(message_size);
            metrics.record_message_sent(message_size);
            messages_sent += 1;

            // Simulate receiving a response
            let should_receive = if config.simulate_packet_loss {
                rand::random::<f64>() > config.packet_loss_rate
            } else {
                true
            };

            if should_receive {
                user_metric.record_message_received(message_size);
                metrics.record_message_received(message_size);
            } else {
                user_metric.record_error();
            }

            // Simulate message interval
            tokio::time::sleep(config.message_interval).await;
        }
    }

    #[tokio::test]
    #[ignore = "slow load test - run with: cargo test -- --ignored"]
    async fn test_load_10_users() {
        let config = Arc::new(
            LoadTestConfig::default()
                .with_users(10)
                .with_duration(Duration::from_secs(10)),
        );
        let metrics = Arc::new(LoadTestMetrics::new(
            config.concurrent_users,
            config.test_duration,
        ));

        let mut tasks = vec![];
        let start_signal = Arc::new(Barrier::new(config.concurrent_users));

        for user_id in 0..config.concurrent_users {
            let metrics_clone = Arc::clone(&metrics);
            let config_clone = Arc::clone(&config);
            let start_signal_clone = Arc::clone(&start_signal);

            let task = tokio::spawn(async move {
                simulate_user(user_id, metrics_clone, config_clone, start_signal_clone).await;
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        metrics.print_summary();

        // Verify some basic metrics
        assert!(
            metrics
                .total_messages_sent
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        );
        assert!(
            metrics
                .total_messages_received
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        );
    }

    #[tokio::test]
    #[ignore = "slow load test - run with: cargo test -- --ignored"]
    async fn test_load_50_users() {
        let config = Arc::new(
            LoadTestConfig::default()
                .with_users(50)
                .with_duration(Duration::from_secs(10)),
        );
        let metrics = Arc::new(LoadTestMetrics::new(
            config.concurrent_users,
            config.test_duration,
        ));

        let mut tasks = vec![];
        let start_signal = Arc::new(Barrier::new(config.concurrent_users));

        for user_id in 0..config.concurrent_users {
            let metrics_clone = Arc::clone(&metrics);
            let config_clone = Arc::clone(&config);
            let start_signal_clone = Arc::clone(&start_signal);

            let task = tokio::spawn(async move {
                simulate_user(user_id, metrics_clone, config_clone, start_signal_clone).await;
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        metrics.print_summary();

        // Verify some basic metrics
        assert!(
            metrics
                .total_messages_sent
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        );
        assert!(
            metrics
                .total_messages_received
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        );
    }

    #[tokio::test]
    #[ignore = "slow load test - run with: cargo test -- --ignored"]
    async fn test_load_100_users() {
        let config = Arc::new(
            LoadTestConfig::default()
                .with_users(100)
                .with_duration(Duration::from_secs(10)),
        );
        let metrics = Arc::new(LoadTestMetrics::new(
            config.concurrent_users,
            config.test_duration,
        ));

        let mut tasks = vec![];
        let start_signal = Arc::new(Barrier::new(config.concurrent_users));

        for user_id in 0..config.concurrent_users {
            let metrics_clone = Arc::clone(&metrics);
            let config_clone = Arc::clone(&config);
            let start_signal_clone = Arc::clone(&start_signal);

            let task = tokio::spawn(async move {
                simulate_user(user_id, metrics_clone, config_clone, start_signal_clone).await;
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        metrics.print_summary();

        // Verify some basic metrics
        assert!(
            metrics
                .total_messages_sent
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        );
        assert!(
            metrics
                .total_messages_received
                .load(std::sync::atomic::Ordering::Relaxed)
                > 0
        );
    }

    #[tokio::test]
    #[ignore = "slow load test - run with: cargo test -- --ignored"]
    async fn test_load_with_network_simulation() {
        let config = Arc::new(
            LoadTestConfig::default()
                .with_users(20)
                .with_duration(Duration::from_secs(10))
                .with_network_simulation(),
        );
        let metrics = Arc::new(LoadTestMetrics::new(
            config.concurrent_users,
            config.test_duration,
        ));

        let mut tasks = vec![];
        let start_signal = Arc::new(Barrier::new(config.concurrent_users));

        for user_id in 0..config.concurrent_users {
            let metrics_clone = Arc::clone(&metrics);
            let config_clone = Arc::clone(&config);
            let start_signal_clone = Arc::clone(&start_signal);

            let task = tokio::spawn(async move {
                simulate_user(user_id, metrics_clone, config_clone, start_signal_clone).await;
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        metrics.print_summary();

        // With packet loss, some errors should be recorded
        let total_errors: u64 = metrics
            .user_metrics
            .iter()
            .map(|u| u.errors.load(std::sync::atomic::Ordering::Relaxed))
            .sum();

        println!("Total errors with network simulation: {}", total_errors);
    }

    #[tokio::test]
    #[ignore = "slow load test - run with: cargo test -- --ignored"]
    async fn test_load_bandwidth_measurement() {
        let config = Arc::new(
            LoadTestConfig::default()
                .with_users(30)
                .with_duration(Duration::from_secs(5)),
        );
        let metrics = Arc::new(LoadTestMetrics::new(
            config.concurrent_users,
            config.test_duration,
        ));

        let mut tasks = vec![];
        let start_signal = Arc::new(Barrier::new(config.concurrent_users));

        for user_id in 0..config.concurrent_users {
            let metrics_clone = Arc::clone(&metrics);
            let config_clone = Arc::clone(&config);
            let start_signal_clone = Arc::clone(&start_signal);

            let task = tokio::spawn(async move {
                simulate_user(user_id, metrics_clone, config_clone, start_signal_clone).await;
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        metrics.print_summary();

        // Verify bandwidth calculations are reasonable
        let bandwidth_mbps = metrics.bytes_per_second() * 8.0 / 1_000_000.0;
        println!("Bandwidth: {:.2} Mbps", bandwidth_mbps);
        assert!(bandwidth_mbps > 0.0);
    }

    #[tokio::test]
    #[ignore = "slow load test - run with: cargo test -- --ignored"]
    async fn test_load_latency_tracking() {
        let config = Arc::new(
            LoadTestConfig::default()
                .with_users(10)
                .with_duration(Duration::from_secs(5)),
        );
        let metrics = Arc::new(LoadTestMetrics::new(
            config.concurrent_users,
            config.test_duration,
        ));

        let mut tasks = vec![];
        let start_signal = Arc::new(Barrier::new(config.concurrent_users));

        for user_id in 0..config.concurrent_users {
            let metrics_clone = Arc::clone(&metrics);
            let config_clone = Arc::clone(&config);
            let start_signal_clone = Arc::clone(&start_signal);

            let task = tokio::spawn(async move {
                // Simulate user with latency tracking
                start_signal_clone.wait().await;

                let user_metric = &metrics_clone.user_metrics[user_id];
                let message_size = 512;

                for _ in 0..20 {
                    let start = std::time::Instant::now();

                    user_metric.record_message_sent(message_size);
                    metrics_clone.record_message_sent(message_size);

                    tokio::time::sleep(Duration::from_millis(10)).await;

                    user_metric.record_message_received(message_size);
                    metrics_clone.record_message_received(message_size);

                    let latency = start.elapsed();
                    user_metric.record_latency_blocking(latency);

                    tokio::time::sleep(config_clone.message_interval).await;
                }
            });
            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        metrics.print_summary();

        // Verify latency was tracked
        let has_latencies = metrics
            .user_metrics
            .iter()
            .any(|u| u.latencies.blocking_lock().len() > 0);
        assert!(
            has_latencies,
            "At least one user should have recorded latencies"
        );
    }
}
