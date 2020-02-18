use crate::instruments::fundamentals::{buckets::SecondsBuckets, Clock};
use crate::snapshot::Snapshot;

#[derive(Default)]
pub struct Bucket {
    pub sum: i64,
    pub count: u64,
    pub min_max: (i64, i64),
}

impl Bucket {
    pub fn update(&mut self, v: i64) {
        self.min_max = if self.count != 0 {
            let (min, max) = self.min_max;
            (std::cmp::min(min, v), std::cmp::max(max, v))
        } else {
            (v, v)
        };
        self.sum += v;
        self.count += 1;
    }
}

#[derive(Debug, PartialEq)]
pub struct BucketsStats {
    peak: i64,
    peak_min: i64,
    peak_avg: f64,
    bottom: i64,
    bottom_max: i64,
    bottom_avg: f64,
    avg: f64,
}

impl BucketsStats {
    pub fn from_buckets<C: Clock>(buckets: &mut SecondsBuckets<Bucket, C>) -> Option<Self> {
        let mut peak = std::i64::MIN;
        let mut peak_min = std::i64::MAX;
        let mut bottom = std::i64::MAX;
        let mut bottom_max = std::i64::MIN;
        let mut sum_bottom = 0;
        let mut sum_peak = 0;
        let mut total_sum = 0;
        let mut total_count = 0;

        buckets.iter().for_each(
            |Bucket {
                 sum,
                 count,
                 min_max,
             }| {
                if *count != 0 {
                    total_sum += sum;
                    total_count += count;

                    let (min, max) = min_max;

                    peak = std::cmp::max(peak, *max);
                    peak_min = std::cmp::min(peak_min, *max);
                    bottom = std::cmp::min(bottom, *min);
                    bottom_max = std::cmp::max(bottom_max, *min);
                    sum_bottom += min;
                    sum_peak += max;
                }
            },
        );

        if total_count != 0 {
            let avg = (total_sum as f64) / (total_count as f64);
            let bottom_avg = (sum_bottom as f64) / (buckets.len() as f64);
            let peak_avg = (sum_peak as f64) / (buckets.len() as f64);
            Some(BucketsStats {
                peak,
                peak_min,
                peak_avg,
                bottom,
                bottom_max,
                bottom_avg,
                avg,
            })
        } else {
            None
        }
    }

    pub fn add_to_snapshot(self, snapshot: &mut Snapshot, prefix: Option<&str>) {
        use std::borrow::Cow;
        let prefix = if let Some(prefix) = prefix {
            Cow::Owned(format!("{}_", prefix))
        } else {
            Cow::Borrowed("")
        };
        let peak_name = format!("{}peak", prefix);
        snapshot.items.push((peak_name, self.peak.into()));
        let peak_min_name = format!("{}peak_min", prefix);
        snapshot.items.push((peak_min_name, self.peak_min.into()));
        let peak_avg_name = format!("{}peak_avg", prefix);
        snapshot.items.push((peak_avg_name, self.peak_avg.into()));
        let bottom_name = format!("{}bottom", prefix);
        snapshot.items.push((bottom_name, self.bottom.into()));
        let bottom_max_name = format!("{}bottom_max", prefix);
        snapshot
            .items
            .push((bottom_max_name, self.bottom_max.into()));
        let bottom_avg_name = format!("{}bottom_avg", prefix);
        snapshot
            .items
            .push((bottom_avg_name, self.bottom_avg.into()));
        let avg_name = format!("{}avg", prefix);
        snapshot.items.push((avg_name, self.avg.into()));
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod test {
    use crate::instruments::fundamentals::buckets::SecondsBuckets;
    use crate::instruments::fundamentals::ManualOffsetClock;

    use super::*;

    #[test]
    fn empty_bucket_returns_none() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(1, clock);

        assert!(BucketsStats::from_buckets(&mut buckets).is_none());
    }

    #[test]
    fn none_empty_bucket_returns_some() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(1, clock);
        buckets.current_mut().update(1);

        assert!(BucketsStats::from_buckets(&mut buckets).is_some());
    }

    #[test]
    fn one_bucket_works_correctly_with_one_update() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(1, clock);
        buckets.current_mut().update(1);

        let stats = BucketsStats::from_buckets(&mut buckets).unwrap();
        assert_eq!(stats.peak, 1);
        assert_eq!(stats.peak_min, 1);
        assert_eq!(stats.peak_avg, 1.0);
        assert_eq!(stats.bottom, 1);
        assert_eq!(stats.bottom_max, 1);
        assert_eq!(stats.bottom_avg, 1.0);
        assert_eq!(stats.avg, 1.0);
    }

    #[test]
    fn one_bucket_works_correctly_with_two_different_updates_in_the_same_bucket() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(1, clock);
        buckets.current_mut().update(1);
        buckets.current_mut().update(2);

        let stats = BucketsStats::from_buckets(&mut buckets).unwrap();
        assert_eq!(stats.peak, 2, "peak");
        assert_eq!(stats.peak_min, 2, "peak_min");
        assert_eq!(stats.peak_avg, 2.0, "peak_avg");
        assert_eq!(stats.bottom, 1, "bottom");
        assert_eq!(stats.bottom_max, 1, "bottom_max");
        assert_eq!(stats.bottom_avg, 1.0, "bottom_avg");
        assert_eq!(stats.avg, 1.5, "avg");
    }

    #[test]
    fn one_bucket_works_correctly_with_2_times_the_same_updates_and_one_bucket_shift() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(1, clock.clone());
        buckets.current_mut().update(1);
        clock.advance_a_second();
        buckets.current_mut().update(1);

        let stats = BucketsStats::from_buckets(&mut buckets).unwrap();
        assert_eq!(stats.peak, 1, "peak");
        assert_eq!(stats.peak_min, 1, "peak_min");
        assert_eq!(stats.peak_avg, 1.0, "peak_avg");
        assert_eq!(stats.bottom, 1, "bottom");
        assert_eq!(stats.bottom_max, 1, "bottom_max");
        assert_eq!(stats.bottom_avg, 1.0, "bottom_avg");
        assert_eq!(stats.avg, 1.0, "avg");
    }

    #[test]
    fn one_bucket_works_correctly_with_2_times_a_different_update_and_one_bucket_shift() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(1, clock.clone());
        buckets.current_mut().update(1);
        clock.advance_a_second();
        buckets.current_mut().update(2);

        let stats = BucketsStats::from_buckets(&mut buckets).unwrap();
        assert_eq!(stats.peak, 2, "peak");
        assert_eq!(stats.peak_min, 2, "peak_min");
        assert_eq!(stats.peak_avg, 2.0, "peak_avg");
        assert_eq!(stats.bottom, 2, "bottom");
        assert_eq!(stats.bottom_max, 2, "bottom_max");
        assert_eq!(stats.bottom_avg, 2.0, "bottom_avg");
        assert_eq!(stats.avg, 2.0, "avg");
    }

    #[test]
    fn two_buckets_works_correctly_with_2_times_the_same_updates_and_one_bucket_shift() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(2, clock.clone());
        buckets.current_mut().update(1);
        clock.advance_a_second();
        buckets.current_mut().update(1);

        let stats = BucketsStats::from_buckets(&mut buckets).unwrap();
        assert_eq!(stats.peak, 1, "peak");
        assert_eq!(stats.peak_min, 1, "peak_min");
        assert_eq!(stats.peak_avg, 1.0, "peak_avg");
        assert_eq!(stats.bottom, 1, "bottom");
        assert_eq!(stats.bottom_max, 1, "bottom_max");
        assert_eq!(stats.bottom_avg, 1.0, "bottom_avg");
        assert_eq!(stats.avg, 1.0, "avg");
    }

    #[test]
    fn two_buckets_works_correctly_with_different_updates_and_one_bucket_shift() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(2, clock.clone());
        buckets.current_mut().update(1);
        clock.advance_a_second();
        buckets.current_mut().update(2);

        let stats = BucketsStats::from_buckets(&mut buckets).unwrap();
        assert_eq!(stats.peak, 2, "peak");
        assert_eq!(stats.peak_min, 1, "peak_min");
        assert_eq!(stats.peak_avg, 1.5, "peak_avg");
        assert_eq!(stats.bottom, 1, "bottom");
        assert_eq!(stats.bottom_max, 2, "bottom_max");
        assert_eq!(stats.bottom_avg, 1.5, "bottom_avg");
        assert_eq!(stats.avg, 1.5, "avg");
    }

    #[test]
    fn two_buckets_works_correctly_2_different_updates_in_each_bucket_with_one_bucket_shift() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<Bucket, _>::with_clock(2, clock.clone());
        buckets.current_mut().update(2);
        buckets.current_mut().update(5);
        clock.advance_a_second();
        buckets.current_mut().update(4);
        buckets.current_mut().update(7);

        let stats = BucketsStats::from_buckets(&mut buckets).unwrap();
        assert_eq!(stats.peak, 7, "peak");
        assert_eq!(stats.peak_min, 5, "peak_min");
        assert_eq!(stats.peak_avg, 6.0, "peak_avg");
        assert_eq!(stats.bottom, 2, "bottom");
        assert_eq!(stats.bottom_max, 4, "bottom_max");
        assert_eq!(stats.bottom_avg, 3.0, "bottom_avg");
        assert_eq!(stats.avg, 4.5, "avg");
    }
}
