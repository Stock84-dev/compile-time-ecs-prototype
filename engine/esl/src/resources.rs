pub use crate::types::{Fee, Slippage};

pub struct LoopEndBoundExcluded(pub usize);
pub struct AccountsPerThread(pub usize);
pub struct ThreadsPerDevice(pub usize);
pub struct ThreadId(pub usize);
pub struct NSamples(pub usize);
pub struct NTrackers(pub usize);
pub struct MetricsPtr(pub *mut u8);
pub struct TracksPtr(pub *mut u8);
pub struct StartingBalance(pub f32);
pub struct TradingDaysPerYear(pub f32);
/// Yearly bond yield (not in percentages)
pub struct RiskFreeRate(pub f32);
/// Elapsed time of a simulation in nanoseconds. e.g. `current_candle_index * timeframe_ns`.
pub struct Elapsed(pub u64);
impl Elapsed {
    pub fn years(&self) -> f32 {
        self.seconds() / 60. / 60. / 24. / 365.
    }

    pub fn seconds(&self) -> f32 {
        self.0 as f32 / 1_000_000_000. 
    }
}
pub struct TimeframeS(pub u32);
pub struct StartTimestampNs(pub u64);
// pub struct TimestampMS(pub u64);
