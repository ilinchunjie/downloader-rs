use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{sleep, Duration, Instant};

/// A token-bucket rate limiter shared across all download tasks.
/// When `bytes_per_second` is 0, no rate limiting is applied.
pub struct RateLimiter {
    bytes_per_second: u64,
    tokens: AtomicU64,
    last_refill: std::sync::Mutex<Instant>,
}

impl RateLimiter {
    pub fn new(bytes_per_second: u64) -> Arc<Self> {
        Arc::new(Self {
            bytes_per_second,
            tokens: AtomicU64::new(bytes_per_second),
            last_refill: std::sync::Mutex::new(Instant::now()),
        })
    }

    /// Returns true if rate limiting is disabled (unlimited speed).
    pub fn is_unlimited(&self) -> bool {
        self.bytes_per_second == 0
    }

    /// Consume `amount` bytes from the bucket. If not enough tokens,
    /// sleep until enough have been refilled.
    pub async fn acquire(&self, amount: u64) {
        if self.is_unlimited() {
            return;
        }

        loop {
            // Refill tokens based on elapsed time
            self.refill();

            let current = self.tokens.load(Ordering::Relaxed);
            if current >= amount {
                // Try to consume tokens
                match self.tokens.compare_exchange(
                    current,
                    current - amount,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => return,
                    Err(_) => continue, // lost race, retry
                }
            } else {
                // Not enough tokens, sleep a short duration then retry
                let needed = amount - current;
                let wait_secs = needed as f64 / self.bytes_per_second as f64;
                let wait = Duration::from_secs_f64(wait_secs.min(0.1)); // cap wait at 100ms
                sleep(wait).await;
            }
        }
    }

    fn refill(&self) {
        let mut last = self.last_refill.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(*last);
        let new_tokens = (elapsed.as_secs_f64() * self.bytes_per_second as f64) as u64;
        if new_tokens > 0 {
            *last = now;
            let current = self.tokens.load(Ordering::Relaxed);
            let capped = (current + new_tokens).min(self.bytes_per_second);
            self.tokens.store(capped, Ordering::Relaxed);
        }
    }
}
