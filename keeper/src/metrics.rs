use std::sync::Mutex;
use std::time::Instant;
use tracing::info;

/// Thread-safe metrics tracking for the keeper bot.
pub struct Metrics {
    inner: Mutex<MetricsInner>,
}

struct MetricsInner {
    positions_monitored: usize,
    liquidations_executed: u64,
    liquidations_failed: u64,
    funding_settlements: u64,
    funding_failures: u64,
    started_at: Instant,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MetricsInner {
                positions_monitored: 0,
                liquidations_executed: 0,
                liquidations_failed: 0,
                funding_settlements: 0,
                funding_failures: 0,
                started_at: Instant::now(),
            }),
        }
    }

    pub fn set_positions_monitored(&self, count: usize) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.positions_monitored = count;
        }
    }

    pub fn record_liquidation(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.liquidations_executed += 1;
        }
    }

    pub fn record_liquidation_failure(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.liquidations_failed += 1;
        }
    }

    pub fn record_funding_settlement(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.funding_settlements += 1;
        }
    }

    pub fn record_funding_failure(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.funding_failures += 1;
        }
    }

    pub fn log_summary(&self) {
        if let Ok(inner) = self.inner.lock() {
            let uptime = inner.started_at.elapsed();
            info!(
                "Keeper metrics: uptime={:.0}s positions={} liquidations={}/{} funding={}/{}",
                uptime.as_secs_f64(),
                inner.positions_monitored,
                inner.liquidations_executed,
                inner.liquidations_failed,
                inner.funding_settlements,
                inner.funding_failures,
            );
        }
    }
}
