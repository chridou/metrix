use std::time::Instant;

use super::{Clock, WallClock};

pub struct SecondsBuckets<T, C = WallClock> {
    buckets: Vec<T>,
    clock: C,
    last_tick: Instant,
    aggregated: T,
}

impl<T> SecondsBuckets<T, WallClock>
where
    T: Default + Copy + std::ops::Add + std::ops::Sub,
{
    fn new(for_seconds: usize) -> Self {
        Self::with_clock(for_seconds, WallClock)
    }
}

impl<T, C> SecondsBuckets<T, C>
where
    T: Default + Copy + std::ops::Add + std::ops::Sub,
    C: Clock,
{
    pub fn with_clock(for_seconds: usize, clock: C) -> Self {
        if for_seconds == 0 {
            panic!("for_seconds must be greater than 0");
        }

        let last_tick = clock.now();
        let buckets = (0..for_seconds).map(|_| T::default()).collect();
        Self {
            buckets,
            clock,
            last_tick,
            aggregated: T::default(),
        }
    }
}
