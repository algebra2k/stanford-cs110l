use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

struct RequestState {
    last_time: Instant,
    requests: usize,
}

impl Default for RequestState {
    fn default() -> Self {
        RequestState {
            last_time: Instant::now(),
            requests: 1,
        }
    }
}

pub struct FixWindowRateLimit {
    max_requests_per_minute: usize,
    requests_per_ip: Mutex<HashMap<String, RequestState>>,
}

impl FixWindowRateLimit {
    pub fn new(max_requests_per_minute: usize) -> FixWindowRateLimit {
        FixWindowRateLimit {
            max_requests_per_minute,
            requests_per_ip: Mutex::new(HashMap::new()),
        }
    }

    pub async fn rate_limit(&mut self, ip: &str) -> bool {
        if self.max_requests_per_minute == 0 {
            return false;
        }

        let mut hm = self.requests_per_ip.lock().await;
        let requests_per_ip = match hm.get_mut(ip) {
            Some(entry) => entry,
            None => {
                hm.insert(ip.to_string(), Default::default());
                return false;
            }
        };

        if requests_per_ip.last_time + Duration::from_secs(60) < Instant::now() {
            requests_per_ip.last_time = Instant::now();
            requests_per_ip.requests = 0;
            return false;
        }

        requests_per_ip.requests += 1;
        if requests_per_ip.requests > self.max_requests_per_minute {
            return true;
        }

        return false;
    }
}
