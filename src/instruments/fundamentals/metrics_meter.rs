// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::sync::Mutex;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

const NANOS_PER_SEC: u64 = 1_000_000_000;
const WINDOW: [f64; 3] = [1.0, 5.0, 15.0];

// A MeterSnapshot
#[derive(Debug)]
pub struct MeterSnapshot {
    pub count: i64,
    pub rates: [f64; 3],
    pub mean: f64,
}

#[derive(Debug)]
struct StdMeterData {
    count: i64,
    ewma: [EWMA; 3],
    next_tick: Instant,
}

// A StdMeter struct
#[derive(Debug)]
pub struct StdMeter {
    data: Mutex<StdMeterData>,
    start: Instant,
}

// A Meter trait
pub trait Meter: Send + Sync {
    fn snapshot(&self) -> MeterSnapshot;

    fn mark(&self, n: i64);

    fn tick(&self);

    fn rate(&self, rate: f64) -> f64;

    fn mean(&self) -> f64;

    fn count(&self) -> i64;
}

impl Meter for StdMeter {
    fn snapshot(&self) -> MeterSnapshot {
        let mut s = self.data.lock().unwrap();
        self.tick_inner(&mut s);

        MeterSnapshot {
            count: s.count,
            rates: [s.ewma[0].rate(), s.ewma[1].rate(), s.ewma[2].rate()],
            mean: self.mean_inner(&s),
        }
    }

    fn mark(&self, n: i64) {
        let mut s = self.data.lock().unwrap();
        self.tick_inner(&mut s);

        s.count += n;

        for i in 0..WINDOW.len() {
            s.ewma[i].update(n as usize);
        }
    }

    fn tick(&self) {
        let mut s = self.data.lock().unwrap();
        self.tick_inner(&mut s);
    }

    /// Return the given EWMA for a rate like 1, 5, 15 minutes
    #[allow(clippy::float_cmp)]
    fn rate(&self, rate: f64) -> f64 {
        let mut s = self.data.lock().unwrap();
        self.tick_inner(&mut s);

        if let Some(pos) = WINDOW.iter().position(|w| *w == rate) {
            return s.ewma[pos].rate();
        }
        0.0
    }

    /// Return the mean rate
    fn mean(&self) -> f64 {
        let s = self.data.lock().unwrap();
        self.mean_inner(&s)
    }

    fn count(&self) -> i64 {
        let s = self.data.lock().unwrap();

        s.count
    }
}

impl StdMeter {
    #[cfg(test)]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn mean_inner(&self, s: &StdMeterData) -> f64 {
        if s.count == 0 {
            0.
        } else {
            let dur = self.start.elapsed();
            let nanos = dur.as_secs() * NANOS_PER_SEC + dur.subsec_nanos() as u64;
            s.count as f64 / nanos as f64 * NANOS_PER_SEC as f64
        }
    }

    fn tick_inner(&self, s: &mut StdMeterData) {
        let now = Instant::now();

        while s.next_tick <= now {
            for ewma in &mut s.ewma {
                ewma.tick();
            }
            s.next_tick += Duration::from_secs(TICK_RATE_SECS);
        }
    }
}

impl Default for StdMeter {
    fn default() -> Self {
        let now = Instant::now();
        StdMeter {
            data: Mutex::new(StdMeterData {
                count: 0,
                ewma: [EWMA::new(1.0), EWMA::new(5.0), EWMA::new(15.0)],
                next_tick: now + Duration::from_secs(TICK_RATE_SECS),
            }),
            start: now,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn zero() {
        let m = StdMeter::new();
        let s = m.snapshot();

        assert_eq!(s.count, 0);
    }

    #[test]
    fn non_zero() {
        let m = StdMeter::new();
        m.mark(3);

        let s = m.snapshot();

        assert_eq!(s.count, 3);
    }

    #[test]
    fn snapshot() {
        let m = StdMeter::new();
        m.mark(1);
        m.mark(1);

        let s = m.snapshot();

        m.mark(1);

        assert_eq!(s.count, 2);
        assert_eq!(m.snapshot().count, 3);
    }

    // Test that decay works correctly
    #[test]
    fn decay() {
        let m = StdMeter::new();

        m.tick();
    }
}

/// EWMA ==================================0

/// The rate in seconds at which `EWMA::tick` should be called.
pub const TICK_RATE_SECS: u64 = 5;

/// An exponentially weighted moving average.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct EWMA {
    uncounted: AtomicUsize, // This tracks uncounted events
    alpha: f64,
    rate: f64,
    init: bool,
}

#[allow(missing_docs)]
impl EWMA {
    pub fn rate(&self) -> f64 {
        self.rate * (NANOS_PER_SEC as f64)
    }

    pub fn tick(&mut self) {
        let counter = self.uncounted.swap(0, Ordering::SeqCst);
        let i_rate = (counter as f64) / (TICK_RATE_SECS * NANOS_PER_SEC) as f64;

        if self.init {
            self.rate += self.alpha * (i_rate - self.rate);
        } else {
            self.init = true;
            self.rate = i_rate;
        }
    }

    pub fn update(&self, n: usize) {
        self.uncounted.fetch_add(n, Ordering::SeqCst);
    }

    /// construct new by alpha
    pub fn new_by_alpha(alpha: f64) -> Self {
        EWMA {
            uncounted: AtomicUsize::new(0),
            alpha,
            rate: 0.0,
            init: false,
        }
    }

    /// constructs a new EWMA for a n-minute moving average.
    pub fn new(n: f64) -> Self {
        let i = -(TICK_RATE_SECS as f64) / 60.0 / n;
        EWMA::new_by_alpha(1.0 - i.exp())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
#[allow(clippy::unreadable_literal)]
#[allow(clippy::excessive_precision)]
mod test_ewma {
    use super::*;

    // Tick a minute
    fn elapse_minute(e: &mut EWMA) {
        for _ in 0..12 {
            e.tick();
        }
    }

    // Returns whether the rate() is within 0.0001 of expected after ticking a minute
    fn within(e: &mut EWMA, expected: f64) -> bool {
        elapse_minute(e);
        let r = e.rate();
        (r - expected).abs() < 0.0001
    }

    #[test]
    fn ewma1() {
        let mut e = EWMA::new(1.0);
        e.update(3);
        e.tick();

        // initial
        let r = e.rate();
        assert_eq!(r, 0.6);

        // 1 minute
        assert_eq!(within(&mut e, 0.22072766470286553), true);

        // 2 minute
        assert_eq!(within(&mut e, 0.08120116994196772), true);

        // 3 minute
        assert_eq!(within(&mut e, 0.029872241020718428), true);

        // 4 minute
        assert_eq!(within(&mut e, 0.01098938333324054), true);

        // 5 minute
        assert_eq!(within(&mut e, 0.004042768199451294), true);

        // 6 minute
        assert_eq!(within(&mut e, 0.0014872513059998212), true);

        // 7 minute
        assert_eq!(within(&mut e, 0.0005471291793327122), true);

        // 8 minute
        assert_eq!(within(&mut e, 0.00020127757674150815), true);

        // 9 minute
        assert_eq!(within(&mut e, 7.404588245200814e-05), true);

        // 10 minute
        assert_eq!(within(&mut e, 2.7239957857491083e-05), true);

        // 11 minute
        assert_eq!(within(&mut e, 1.0021020474147462e-05), true);

        // 12 minute
        assert_eq!(within(&mut e, 3.6865274119969525e-06), true);

        // 13 minute
        assert_eq!(within(&mut e, 1.3561976441886433e-06), true);

        // 14 minute
        assert_eq!(within(&mut e, 4.989172314621449e-07), true);

        // 15 minute
        assert_eq!(within(&mut e, 1.8354139230109722e-07), true);
    }

    #[test]
    fn ewma5() {
        let mut e = EWMA::new(5.0);
        e.update(3);
        e.tick();

        let r = e.rate();
        assert_eq!(r, 0.6);

        // 1 minute
        assert_eq!(within(&mut e, 0.49123845184678905), true);

        // 2 minute
        assert_eq!(within(&mut e, 0.4021920276213837), true);

        // 3 minute
        assert_eq!(within(&mut e, 0.32928698165641596), true);

        // 4 minute
        assert_eq!(within(&mut e, 0.269597378470333), true);

        // 5 minute
        assert_eq!(within(&mut e, 0.2207276647028654), true);

        // 6 minute
        assert_eq!(within(&mut e, 0.18071652714732128), true);

        // 7 minute
        assert_eq!(within(&mut e, 0.14795817836496392), true);

        // 8 minute
        assert_eq!(within(&mut e, 0.12113791079679326), true);

        // 9 minute
        assert_eq!(within(&mut e, 0.09917933293295193), true);

        // 10 minute
        assert_eq!(within(&mut e, 0.08120116994196763), true);

        // 11 minute
        assert_eq!(within(&mut e, 0.06648189501740036), true);

        // 12 minute
        assert_eq!(within(&mut e, 0.05443077197364752), true);

        // 13 minute
        assert_eq!(within(&mut e, 0.04456414692860035), true);

        // 14 minute
        assert_eq!(within(&mut e, 0.03648603757513079), true);

        // 15 minute
        assert_eq!(within(&mut e, 0.0298722410207183831020718428), true);
    }

    #[test]
    fn ewma15() {
        let mut e = EWMA::new(15.0);
        e.update(3);
        e.tick();

        let r = e.rate();
        assert_eq!(r, 0.6);

        // 1 minute
        assert_eq!(within(&mut e, 0.5613041910189706), true);

        // 2 minute
        assert_eq!(within(&mut e, 0.5251039914257684), true);

        // 3 minute
        assert_eq!(within(&mut e, 0.4912384518467888184678905), true);

        // 4 minute
        assert_eq!(within(&mut e, 0.459557003018789), true);

        // 5 minute
        assert_eq!(within(&mut e, 0.4299187863442732), true);

        // 6 minute
        assert_eq!(within(&mut e, 0.4021920276213831), true);

        // 7 minute
        assert_eq!(within(&mut e, 0.37625345116383313), true);

        // 8 minute
        assert_eq!(within(&mut e, 0.3519877317060185), true);

        // 9 minute
        assert_eq!(within(&mut e, 0.3292869816564153165641596), true);

        // 10 minute
        assert_eq!(within(&mut e, 0.3080502714195546), true);

        // 11 minute
        assert_eq!(within(&mut e, 0.2881831806538789), true);

        // 12 minute
        assert_eq!(within(&mut e, 0.26959737847033216), true);

        // 13 minute
        assert_eq!(within(&mut e, 0.2522102307052083), true);

        // 14 minute
        assert_eq!(within(&mut e, 0.23594443252115815), true);

        // 15 minute
        assert_eq!(within(&mut e, 0.2207276647028646247028654470286553), true);
    }
}
