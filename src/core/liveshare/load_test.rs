//! Load testing module for LiveShare
//!
//! This module provides infrastructure for load testing the LiveShare system
//! with multiple concurrent users, measuring performance metrics such as:
//! - Latency (response time, message delivery time)
//! - Bandwidth usage
//! - CPU and memory usage
//! - Message throughput
//! - Connection handling under load

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Load test metrics collection
#[derive(Clone, Debug)]
pub struct LoadTestMetrics {
    /// Number of concurrent users
    pub concurrent_users: usize,
    /// Total messages sent
    pub total_messages_sent: Arc<AtomicU64>,
    /// Total messages received
    pub total_messages_received: Arc<AtomicU64>,
    /// Total bytes sent
    pub total_bytes_sent: Arc<AtomicU64>,
    /// Total bytes received
    pub total_bytes_received: Arc<AtomicU64>,
    /// Start time of the test
    pub start_time: Instant,
    /// Test duration
    pub duration: Duration,
    /// Individual user metrics
    pub user_metrics: Vec<UserMetrics>,
}

/// Per-user metrics
#[derive(Clone, Debug)]
pub struct UserMetrics {
    pub user_id: Uuid,
    pub username: String,
    pub messages_sent: Arc<AtomicU64>,
    pub messages_received: Arc<AtomicU64>,
    pub bytes_sent: Arc<AtomicU64>,
    pub bytes_received: Arc<AtomicU64>,
    pub latencies: Arc<tokio::sync::Mutex<Vec<Duration>>>,
    pub errors: Arc<AtomicU64>,
}

impl UserMetrics {
    pub fn new(user_id: Uuid, username: String) -> Self {
        Self {
            user_id,
            username,
            messages_sent: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            latencies: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            errors: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn record_message_sent(&self, bytes: u64) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_message_received(&self, bytes: u64) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_latency_blocking(&self, latency: Duration) {
        self.latencies.blocking_lock().push(latency);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_average_latency(&self) -> Option<Duration> {
        let latencies = self.latencies.blocking_lock();
        if latencies.is_empty() {
            return None;
        }
        let sum: Duration = latencies.iter().sum();
        Some(sum / latencies.len() as u32)
    }

    pub fn get_max_latency(&self) -> Option<Duration> {
        self.latencies.blocking_lock().iter().max().copied()
    }

    pub fn get_min_latency(&self) -> Option<Duration> {
        self.latencies.blocking_lock().iter().min().copied()
    }
}

impl LoadTestMetrics {
    pub fn new(concurrent_users: usize, duration: Duration) -> Self {
        let user_metrics = (0..concurrent_users)
            .map(|i| UserMetrics::new(Uuid::new_v4(), format!("loadtest_user_{}", i)))
            .collect();

        Self {
            concurrent_users,
            total_messages_sent: Arc::new(AtomicU64::new(0)),
            total_messages_received: Arc::new(AtomicU64::new(0)),
            total_bytes_sent: Arc::new(AtomicU64::new(0)),
            total_bytes_received: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            duration,
            user_metrics,
        }
    }

    pub fn record_message_sent(&self, bytes: u64) {
        self.total_messages_sent.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn record_message_received(&self, bytes: u64) {
        self.total_messages_received.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_received
            .fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn is_complete(&self) -> bool {
        self.elapsed() >= self.duration
    }

    pub fn messages_per_second(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_messages_sent.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn bytes_per_second(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.total_bytes_sent.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        }
    }

    pub fn print_summary(&self) {
        println!("\n========== LOAD TEST RESULTS ==========");
        println!("Concurrent Users: {}", self.concurrent_users);
        println!("Test Duration: {:?}", self.duration);
        println!("Actual Duration: {:?}", self.elapsed());
        println!("\n--- Overall Metrics ---");
        println!(
            "Total Messages Sent: {}",
            self.total_messages_sent.load(Ordering::Relaxed)
        );
        println!(
            "Total Messages Received: {}",
            self.total_messages_received.load(Ordering::Relaxed)
        );
        println!(
            "Total Bytes Sent: {} ({:.2} MB)",
            self.total_bytes_sent.load(Ordering::Relaxed),
            self.total_bytes_sent.load(Ordering::Relaxed) as f64 / 1_048_576.0
        );
        println!(
            "Total Bytes Received: {} ({:.2} MB)",
            self.total_bytes_received.load(Ordering::Relaxed),
            self.total_bytes_received.load(Ordering::Relaxed) as f64 / 1_048_576.0
        );
        println!("Messages/sec: {:.2}", self.messages_per_second());
        println!(
            "Bandwidth (sent): {:.2} Mbps",
            self.bytes_per_second() * 8.0 / 1_000_000.0
        );

        println!("\n--- Per-User Average Metrics ---");
        if !self.user_metrics.is_empty() {
            let avg_sent = self.total_messages_sent.load(Ordering::Relaxed) as f64
                / self.concurrent_users as f64;
            let avg_recv = self.total_messages_received.load(Ordering::Relaxed) as f64
                / self.concurrent_users as f64;
            let avg_bytes_sent =
                self.total_bytes_sent.load(Ordering::Relaxed) as f64 / self.concurrent_users as f64;
            let avg_bytes_recv = self.total_bytes_received.load(Ordering::Relaxed) as f64
                / self.concurrent_users as f64;

            println!("Avg Messages Sent per User: {:.2}", avg_sent);
            println!("Avg Messages Received per User: {:.2}", avg_recv);
            println!("Avg Bytes Sent per User: {:.2}", avg_bytes_sent);
            println!("Avg Bytes Received per User: {:.2}", avg_bytes_recv);

            // Calculate latency statistics
            let mut all_latencies = Vec::new();
            for user in &self.user_metrics {
                all_latencies.extend(user.latencies.blocking_lock().iter().copied());
            }

            if !all_latencies.is_empty() {
                all_latencies.sort();
                let avg_latency =
                    all_latencies.iter().sum::<Duration>() / all_latencies.len() as u32;
                let p50 = all_latencies[all_latencies.len() / 2];
                let p95 = all_latencies[(all_latencies.len() * 95) / 100];
                let p99 = all_latencies[(all_latencies.len() * 99) / 100];

                println!("\n--- Latency Statistics ---");
                println!("Average Latency: {:?}", avg_latency);
                println!("p50 Latency: {:?}", p50);
                println!("p95 Latency: {:?}", p95);
                println!("p99 Latency: {:?}", p99);
            }

            // Error statistics
            let total_errors: u64 = self
                .user_metrics
                .iter()
                .map(|u| u.errors.load(Ordering::Relaxed))
                .sum();
            if total_errors > 0 {
                println!("Total Errors: {}", total_errors);
                println!(
                    "Error Rate: {:.2}%",
                    (total_errors as f64
                        / (self.total_messages_sent.load(Ordering::Relaxed) as f64
                            + total_errors as f64))
                        * 100.0
                );
            }
        }
        println!("========================================\n");
    }
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub concurrent_users: usize,
    pub messages_per_user: usize,
    pub message_interval: Duration,
    pub test_duration: Duration,
    pub simulate_network_latency: bool,
    pub simulate_packet_loss: bool,
    pub packet_loss_rate: f64,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            concurrent_users: 10,
            messages_per_user: 100,
            message_interval: Duration::from_millis(100),
            test_duration: Duration::from_secs(60),
            simulate_network_latency: false,
            simulate_packet_loss: false,
            packet_loss_rate: 0.0,
        }
    }
}

impl LoadTestConfig {
    pub fn with_users(mut self, users: usize) -> Self {
        self.concurrent_users = users;
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.test_duration = duration;
        self
    }

    pub fn with_network_simulation(mut self) -> Self {
        self.simulate_network_latency = true;
        self.simulate_packet_loss = true;
        self.packet_loss_rate = 0.05; // 5% packet loss
        self
    }

    pub fn with_packet_loss(mut self, rate: f64) -> Self {
        self.simulate_packet_loss = true;
        self.packet_loss_rate = rate.clamp(0.0, 1.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_metrics_creation() {
        let user_id = Uuid::new_v4();
        let metrics = UserMetrics::new(user_id, "test_user".to_string());

        assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.messages_received.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.errors.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_user_metrics_message_recording() {
        let metrics = UserMetrics::new(Uuid::new_v4(), "test_user".to_string());

        metrics.record_message_sent(1024);
        metrics.record_message_received(2048);

        assert_eq!(metrics.messages_sent.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.bytes_sent.load(Ordering::Relaxed), 1024);
        assert_eq!(metrics.messages_received.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.bytes_received.load(Ordering::Relaxed), 2048);
    }

    #[test]
    fn test_user_metrics_latency() {
        let metrics = UserMetrics::new(Uuid::new_v4(), "test_user".to_string());

        metrics.record_latency_blocking(Duration::from_millis(10));
        metrics.record_latency_blocking(Duration::from_millis(20));
        metrics.record_latency_blocking(Duration::from_millis(30));

        assert_eq!(
            metrics.get_average_latency(),
            Some(Duration::from_millis(20))
        );
        assert_eq!(metrics.get_max_latency(), Some(Duration::from_millis(30)));
        assert_eq!(metrics.get_min_latency(), Some(Duration::from_millis(10)));
    }

    #[test]
    fn test_load_test_metrics_creation() {
        let metrics = LoadTestMetrics::new(10, Duration::from_secs(60));

        assert_eq!(metrics.concurrent_users, 10);
        assert_eq!(metrics.user_metrics.len(), 10);
        assert_eq!(metrics.total_messages_sent.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_load_test_config_builder() {
        let config = LoadTestConfig::default()
            .with_users(50)
            .with_duration(Duration::from_secs(120))
            .with_network_simulation();

        assert_eq!(config.concurrent_users, 50);
        assert_eq!(config.test_duration, Duration::from_secs(120));
        assert!(config.simulate_network_latency);
        assert!(config.simulate_packet_loss);
    }

    #[test]
    fn test_load_test_messages_per_second() {
        let metrics = LoadTestMetrics::new(10, Duration::from_secs(60));

        for _ in 0..100 {
            metrics.record_message_sent(1024);
        }

        // Should be > 0 since we've recorded messages
        assert!(metrics.messages_per_second() > 0.0);
    }
}
