//! Other instruments
pub use self::last_occurance_tracker::*;

mod last_occurance_tracker {
    use std::time::Instant;

    use instruments::{Instrument, Update, Updates};
    use snapshot::Snapshot;
    use {Descriptive, PutsSnapshot};
    use util;

    /// Tracks how much many seconds elapsed since the last occurence
    pub struct LastOccuranceTracker {
        name: String,
        title: Option<String>,
        description: Option<String>,
        happened_last: Option<Instant>,
        invert: bool,
        make_none_zero: bool,
    }

    impl LastOccuranceTracker {
        pub fn new_with_defaults<T: Into<String>>(name: T) -> LastOccuranceTracker {
            LastOccuranceTracker {
                name: name.into(),
                title: None,
                description: None,
                happened_last: None,
                invert: false,
                make_none_zero: false,
            }
        }

        /// Gets the name of this `OccuranceTracker`
        pub fn name(&self) -> &str {
            &self.name
        }

        /// Set the name if this `OccuranceTracker`.
        ///
        /// The name is a path segment within a `Snapshot`
        pub fn set_name<T: Into<String>>(&mut self, name: T) {
            self.name = name.into();
        }

        /// Sets the `title` of this `OccuranceTracker`.
        ///
        /// A title can be part of a descriptive `Snapshot`
        pub fn set_title<T: Into<String>>(&mut self, title: T) {
            self.title = Some(title.into())
        }

        /// Sets the `description` of this `OccuranceTracker`.
        ///
        /// A description can be part of a descriptive `Snapshot`
        pub fn set_description<T: Into<String>>(&mut self, description: T) {
            self.description = Some(description.into())
        }

        /// Set whether the current value should be inverted in a snapshot or not
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
        /// was at least one occurance recorded.
        pub fn set_make_none_zero(&mut self, make_zero: bool) {
            self.make_none_zero = make_zero
        }

        /// return whether `make_none_zero` is on or off
        pub fn make_none_zero(&self) -> bool {
            self.make_none_zero
        }

        /// Returns the current state
        pub fn elapsed_since_last_occurance(&self) -> Option<u64> {
            self.happened_last
                .map(|last| (Instant::now() - last).as_secs())
        }
    }

    impl Instrument for LastOccuranceTracker {}

    impl PutsSnapshot for LastOccuranceTracker {
        fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
            util::put_postfixed_descriptives(self, &self.name, into, descriptive);

            if let Some(v) = self.elapsed_since_last_occurance() {
                into.items.push((self.name.clone(), v.into()));
            } else {
                if self.make_none_zero() {
                    into.items.push((self.name.clone(), 0.into()));
                }
            }
        }
    }

    impl Updates for LastOccuranceTracker {
        fn update(&mut self, _: &Update) {
            self.happened_last = Some(Instant::now())
        }
    }

    impl Descriptive for LastOccuranceTracker {
        fn title(&self) -> Option<&str> {
            self.title.as_ref().map(|n| &**n)
        }

        fn description(&self) -> Option<&str> {
            self.description.as_ref().map(|n| &**n)
        }
    }
}
