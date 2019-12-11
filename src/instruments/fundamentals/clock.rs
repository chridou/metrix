use std::time::Instant;

pub trait Clock {
    fn now(&self) -> Instant;
}

#[derive(Debug, Clone, Copy)]
pub struct WallClock;

impl Clock for WallClock {
    #[inline]
    fn now(&self) -> Instant {
        Instant::now()
    }
}

#[cfg(test)]
pub mod manual_clock {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::Clock;

    /// A clock for testing
    ///
    /// Clones will share the offset
    #[derive(Debug, Clone)]
    pub struct ManualOffsetClock {
        reference: Instant,
        offset: Arc<Mutex<Duration>>,
    }

    impl ManualOffsetClock {
        pub fn offset(&self) -> Duration {
            *self.offset.lock().unwrap()
        }

        pub fn reference(&self) -> Instant {
            self.reference
        }

        pub fn set_offset(&self, offset: Duration) {
            *self.offset.lock().unwrap() = offset;
        }

        pub fn reset(&self) {
            self.set_offset(Duration::from_secs(0))
        }

        pub fn advance_by(&self, by: Duration) {
            let mut offset = self.offset.lock().unwrap();
            *offset += by;
        }

        pub fn advance_n_seconds(&self, secs: u64) {
            self.advance_by(Duration::from_secs(secs))
        }

        pub fn advance_a_second(&self) {
            self.advance_n_seconds(1)
        }

        pub fn in_the_past_by(&self, by: Duration) -> Instant {
            self.now() - by
        }

        pub fn seconds_in_the_past(&self, secs: u64) -> Instant {
            self.in_the_past_by(Duration::from_secs(secs))
        }
    }

    impl Clock for ManualOffsetClock {
        fn now(&self) -> Instant {
            self.reference + *self.offset.lock().unwrap()
        }
    }

    impl Default for ManualOffsetClock {
        fn default() -> Self {
            Self {
                reference: Instant::now(),
                offset: Arc::new(Mutex::new(Duration::from_millis(0))),
            }
        }
    }
}
