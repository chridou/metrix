//! Other instruments
pub use self::last_occurrence_tracker::LastOccurrenceTracker;
pub use self::value_meter::ValueMeter;

mod value_meter {
    use std::cell::Cell;
    use std::time::{Duration, Instant};

    use metrics::metrics::{Meter as MMeter, StdMeter};

    use instruments::meter::{MeterRate, MeterSnapshot};
    use instruments::{Instrument, Update, Updates};

    use snapshot::{ItemKind, Snapshot};
    use util;
    use {Descriptive, PutsSnapshot};

    /// A meter that is ticked by values instead of observations
    pub struct ValueMeter {
        name: String,
        title: Option<String>,
        description: Option<String>,
        last_tick: Cell<Instant>,
        inner_meter: StdMeter,
        lower_cutoff: f64,
        one_minute_rate_enabled: bool,
        five_minute_rate_enabled: bool,
        fifteen_minute_rate_enabled: bool,
    }

    impl ValueMeter {
        pub fn new_with_defaults<T: Into<String>>(name: T) -> ValueMeter {
            ValueMeter {
                name: name.into(),
                title: None,
                description: None,
                last_tick: Cell::new(Instant::now()),
                inner_meter: StdMeter::default(),
                lower_cutoff: 0.001,
                one_minute_rate_enabled: true,
                five_minute_rate_enabled: false,
                fifteen_minute_rate_enabled: false,
            }
        }

        pub fn name(&self) -> &str {
            &self.name
        }

        pub fn set_name<T: Into<String>>(&mut self, name: T) {
            self.name = name.into();
        }

        pub fn set_title<T: Into<String>>(&mut self, title: T) {
            self.title = Some(title.into())
        }

        pub fn set_description<T: Into<String>>(&mut self, description: T) {
            self.description = Some(description.into())
        }

        /// Rates nbelow this value will be shown as zero.
        ///
        /// Default is 0.001
        pub fn set_lower_cuttoff(&mut self, cutoff: f64) {
            self.lower_cutoff = cutoff
        }

        /// Enable tracking of one minute rates.
        ///
        /// Default: enabled
        pub fn set_one_minute_rate_enabled(&mut self, enabled: bool) {
            self.one_minute_rate_enabled = enabled;
        }

        /// Enable tracking of five minute rates.
        ///
        /// Default: enabled
        pub fn set_five_minute_rate_enabled(&mut self, enabled: bool) {
            self.five_minute_rate_enabled = enabled;
        }

        /// Enable tracking of one minute rates.
        ///
        /// Default: enabled
        pub fn set_fifteen_minute_rate_enabled(&mut self, enabled: bool) {
            self.fifteen_minute_rate_enabled = enabled;
        }

        fn put_values_into_snapshot(&self, into: &mut Snapshot) {
            if self.last_tick.get().elapsed() >= Duration::from_secs(5) {
                self.inner_meter.tick();
                self.last_tick.set(Instant::now());
            }

            let snapshot = self.inner_meter.snapshot();

            let meter_snapshot = MeterSnapshot {
                count: snapshot.count as u64,
                one_minute: if self.one_minute_rate_enabled {
                    Some(MeterRate {
                        rate: if snapshot.rates[0] < self.lower_cutoff {
                            0.0
                        } else {
                            snapshot.rates[0]
                        },
                    })
                } else {
                    None
                },
                five_minutes: if self.one_minute_rate_enabled {
                    Some(MeterRate {
                        rate: if snapshot.rates[1] < self.lower_cutoff {
                            0.0
                        } else {
                            snapshot.rates[1]
                        },
                    })
                } else {
                    None
                },
                fifteen_minutes: if self.one_minute_rate_enabled {
                    Some(MeterRate {
                        rate: if snapshot.rates[2] < self.lower_cutoff {
                            0.0
                        } else {
                            snapshot.rates[2]
                        },
                    })
                } else {
                    None
                },
            };
            meter_snapshot.put_snapshot(into);
        }
    }

    impl Instrument for ValueMeter {}

    impl PutsSnapshot for ValueMeter {
        fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
            util::put_postfixed_descriptives(self, &self.name, into, descriptive);
            let mut new_level = Snapshot::default();
            self.put_values_into_snapshot(&mut new_level);
            into.push(self.name.clone(), ItemKind::Snapshot(new_level));
        }
    }

    impl Updates for ValueMeter {
        fn update(&mut self, with: &Update) {
            if self.last_tick.get().elapsed() >= Duration::from_secs(5) {
                self.inner_meter.tick();
                self.last_tick.set(Instant::now());
            }

            match *with {
                Update::ObservationWithValue(v, _) => {
                    if v <= ::std::i64::MAX as u64 && v != 0 {
                        self.inner_meter.mark(v as i64)
                    }
                }
                _ => (),
            }
        }
    }

    impl Descriptive for ValueMeter {
        fn title(&self) -> Option<&str> {
            self.title.as_ref().map(|n| &**n)
        }

        fn description(&self) -> Option<&str> {
            self.description.as_ref().map(|n| &**n)
        }
    }

}

mod last_occurrence_tracker {
    use std::time::Instant;

    use instruments::{Instrument, Update, Updates};
    use snapshot::Snapshot;
    use util;
    use {Descriptive, PutsSnapshot};

    /// Tracks how much many seconds elapsed since the last occurence
    pub struct LastOccurrenceTracker {
        name: String,
        title: Option<String>,
        description: Option<String>,
        happened_last: Option<Instant>,
        invert: bool,
        make_none_zero: bool,
    }

    impl LastOccurrenceTracker {
        pub fn new_with_defaults<T: Into<String>>(name: T) -> LastOccurrenceTracker {
            LastOccurrenceTracker {
                name: name.into(),
                title: None,
                description: None,
                happened_last: None,
                invert: false,
                make_none_zero: false,
            }
        }

        /// Gets the name of this `OccurenceTracker`
        pub fn name(&self) -> &str {
            &self.name
        }

        /// Set the name if this `OccurenceTracker`.
        ///
        /// The name is a path segment within a `Snapshot`
        pub fn set_name<T: Into<String>>(&mut self, name: T) {
            self.name = name.into();
        }

        /// Sets the `title` of this `OccurenceTracker`.
        ///
        /// A title can be part of a descriptive `Snapshot`
        pub fn set_title<T: Into<String>>(&mut self, title: T) {
            self.title = Some(title.into())
        }

        /// Sets the `description` of this `OccurenceTracker`.
        ///
        /// A description can be part of a descriptive `Snapshot`
        pub fn set_description<T: Into<String>>(&mut self, description: T) {
            self.description = Some(description.into())
        }

        /// Set whether the current value should be inverted in a snapshot or
        /// not
        ///
        /// Default is `false`
        pub fn set_invert(&mut self, invert: bool) {
            self.invert = invert
        }

        /// The current value should be inverted in a snapshot
        ///
        /// Same as `self.set_invert(true);`
        pub fn enable_invert(&mut self) {
            self.invert = true
        }

        /// If set to `true` possible `None`s that would
        /// be returned will instead be `0`.
        ///
        /// Hint: This instrument will return `None` unless there
        /// was at least one Occurence recorded.
        pub fn set_make_none_zero(&mut self, make_zero: bool) {
            self.make_none_zero = make_zero
        }

        /// return whether `make_none_zero` is on or off
        pub fn make_none_zero(&self) -> bool {
            self.make_none_zero
        }

        /// Returns the current state
        pub fn elapsed_since_last_occurrence(&self) -> Option<u64> {
            self.happened_last
                .map(|last| (Instant::now() - last).as_secs())
        }
    }

    impl Instrument for LastOccurrenceTracker {}

    impl PutsSnapshot for LastOccurrenceTracker {
        fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
            util::put_postfixed_descriptives(self, &self.name, into, descriptive);

            if let Some(v) = self.elapsed_since_last_occurrence() {
                into.items.push((self.name.clone(), v.into()));
            } else {
                if self.make_none_zero() {
                    into.items.push((self.name.clone(), 0.into()));
                }
            }
        }
    }

    impl Updates for LastOccurrenceTracker {
        fn update(&mut self, _: &Update) {
            self.happened_last = Some(Instant::now())
        }
    }

    impl Descriptive for LastOccurrenceTracker {
        fn title(&self) -> Option<&str> {
            self.title.as_ref().map(|n| &**n)
        }

        fn description(&self) -> Option<&str> {
            self.description.as_ref().map(|n| &**n)
        }
    }
}
