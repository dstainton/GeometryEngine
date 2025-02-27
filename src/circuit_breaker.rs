use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct CircuitBreaker {
    failure_threshold: u64,
    reset_timeout: Duration,
    failure_count: AtomicUsize,
    last_failure: Arc<RwLock<Option<Instant>>>,
    state: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Closed = 0,
    Open = 1,
    HalfOpen = 2,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u64, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            failure_count: AtomicUsize::new(0),
            last_failure: Arc::new(RwLock::new(None)),
            state: AtomicU64::new(State::Closed as u64),
        }
    }

    pub async fn call<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Future<Output = Result<T, E>>,
    {
        match self.get_state().await {
            State::Open => {
                Err(anyhow::anyhow!("Circuit breaker is open").into())
            }
            State::HalfOpen | State::Closed => {
                match f.await {
                    Ok(result) => {
                        self.record_success().await;
                        Ok(result)
                    }
                    Err(e) => {
                        self.record_failure().await;
                        Err(e)
                    }
                }
            }
        }
    }

    async fn record_success(&self) {
        if self.get_state().await == State::HalfOpen {
            self.state.store(State::Closed as u64, Ordering::SeqCst);
            self.failure_count.store(0, Ordering::SeqCst);
        }
    }

    async fn record_failure(&self) {
        let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure.write().await = Some(Instant::now());

        if failures >= self.failure_threshold as usize {
            self.state.store(State::Open as u64, Ordering::SeqCst);
        }
    }

    async fn get_state(&self) -> State {
        let current_state = self.state.load(Ordering::SeqCst);
        if current_state == State::Open as u64 {
            if let Some(last_failure) = *self.last_failure.read().await {
                if last_failure.elapsed() >= self.reset_timeout {
                    self.state.store(State::HalfOpen as u64, Ordering::SeqCst);
                    return State::HalfOpen;
                }
            }
        }
        match current_state {
            0 => State::Closed,
            1 => State::Open,
            2 => State::HalfOpen,
            _ => unreachable!(),
        }
    }
} 