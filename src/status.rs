use std::time::{Duration, Instant};

#[derive(PartialEq, Eq)]
struct Timeout {
    start_time: Instant,
    duration: Duration,
}

impl Timeout {
    fn new(duration: Duration) -> Self {
        Timeout {
            start_time: Instant::now(),
            duration,
        }
    }

    fn is_valid(&self) -> bool {
        self.start_time.elapsed() < self.duration
    }
}

#[derive(PartialEq, Eq)]
pub struct Status {
    pub message: String,
    timeout: Option<Timeout>,
}

impl Status {
    pub fn new_with_timeout(message: String, duration: Duration) -> Self {
        Status {
            message,
            timeout: Some(Timeout::new(duration)),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.timeout
            .as_ref()
            .map(|timeout| timeout.is_valid())
            .unwrap_or(true)
    }
}
