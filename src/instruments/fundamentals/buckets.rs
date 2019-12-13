use std::time::{Duration, Instant};

use super::{Clock, WallClock};

/// Structure: [T-N, T-(N-1), ...., T-1 ,NOW]
///
/// Newest elements are added to the right!
pub struct SecondsBuckets<T, C = WallClock> {
    buckets: Vec<T>,
    clock: C,
    current_time: Instant,
    current_idx: usize,
}

impl<T> SecondsBuckets<T, WallClock>
where
    T: Default,
{
    pub fn new(for_seconds: usize) -> Self {
        Self::with_clock(for_seconds, WallClock)
    }
}

impl<T, C> SecondsBuckets<T, C>
where
    T: Default,
    C: Clock,
{
    pub fn with_clock(for_seconds: usize, clock: C) -> Self {
        let buckets = (0..for_seconds).map(|_| T::default()).collect();
        Self::initialized_with_clock(buckets, clock)
    }

    /// The last item in the `vec` will be the current item.
    pub fn initialized_with_clock(buckets: Vec<T>, clock: C) -> Self {
        if buckets.is_empty() {
            panic!("buckets must be for at least 1 seconds");
        }

        let current_idx = buckets.len() - 1;
        let current_time = clock.now();
        Self {
            buckets,
            clock,
            current_time,
            current_idx,
        }
    }

    pub fn current_mut(&mut self) -> &mut T {
        self.tick();
        &mut self.buckets[self.current_idx]
    }

    pub fn get_at_mut(&mut self, for_when: Instant) -> Option<&mut T> {
        self.tick();
        if for_when > self.current_time {
            return None;
        }
        let d = (self.current_time - for_when).as_secs() as usize;
        if d >= self.buckets.len() {
            return None;
        }

        let offset = self.buckets.len() + self.current_idx - d;
        let idx = self.idx(offset);

        Some(&mut self.buckets[idx])
    }

    /// Returns an iterator where the "newest" elements come first.
    ///
    /// So the iterator goes back in time.
    pub fn iter(&mut self) -> BucketIterator<T> {
        self.tick();
        let count = self.buckets.len();
        let idx = self.current_idx;
        BucketIterator {
            buckets: &mut self.buckets,
            count,
            idx,
        }
    }

    fn tick(&mut self) {
        let now = self.clock.now();
        let d = (now - self.current_time).as_secs();
        if d == 0 {
            return;
        }
        self.current_time += Duration::from_secs(d);
        let d = d as usize;

        if d == 1 {
            self.current_idx = self.idx(self.current_idx + 1);
            self.buckets[self.current_idx] = T::default();
            return;
        }

        if d < self.buckets.len() {
            for _ in 0..d {
                self.current_idx = self.idx(self.current_idx + 1);
                self.buckets[self.current_idx] = T::default();
            }
            return;
        }

        if d >= self.buckets.len() {
            self.current_idx = 0;
            self.buckets
                .iter_mut()
                .for_each(|bucket| *bucket = T::default());
            return;
        }
    }

    pub fn len(&self) -> usize {
        self.buckets.len()
    }

    #[inline(always)]
    fn idx(&self, offset: usize) -> usize {
        offset % self.buckets.len()
    }
}

pub struct BucketIterator<'a, T> {
    buckets: &'a Vec<T>,
    count: usize,
    idx: usize,
}

impl<'a, T> Iterator for BucketIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count > 0 {
            let idx_safe = self.idx;
            self.count -= 1;
            self.idx = if self.idx == 0 {
                self.buckets.len() - 1
            } else {
                self.idx - 1
            };
            Some(&self.buckets[idx_safe])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::instruments::fundamentals::ManualOffsetClock;

    use super::*;

    #[test]
    fn buckets_with_one_item_works_without_ticks() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(1, clock.clone());

        assert_eq!(*buckets.current_mut(), 0);

        *buckets.current_mut() = 1;
        assert_eq!(*buckets.current_mut(), 1);
        assert_eq!(buckets.get_at_mut(clock.now()).copied(), Some(1));
        assert_eq!(
            buckets
                .get_at_mut(clock.in_the_past_by(Duration::from_millis(999)))
                .copied(),
            Some(1),
            "+999 millis"
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            None,
            "+1 second"
        );

        let all: Vec<_> = buckets.iter().copied().collect();

        assert_eq!(all, vec![1]);
    }

    #[test]
    fn initial_buckets() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(3, clock.clone());

        assert_eq!(buckets.len(), 3,);

        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
    }

    #[test]
    fn with_three_initialized_items() {
        let clock = ManualOffsetClock::default();
        let mut buckets =
            SecondsBuckets::<u32, _>::initialized_with_clock(vec![1, 2, 3], clock.clone());

        assert_eq!(*buckets.current_mut(), 3);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(3),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(2),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(1),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );

        assert_eq!(vec![3, 2, 1], buckets.iter().copied().collect::<Vec<_>>());
    }

    #[test]
    fn with_three_initialized_items_tick_one_second() {
        let clock = ManualOffsetClock::default();
        let mut buckets =
            SecondsBuckets::<u32, _>::initialized_with_clock(vec![1, 2, 3], clock.clone());

        assert_eq!(*buckets.current_mut(), 3);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(3),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(2),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(1),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
        assert_eq!(vec![3, 2, 1], buckets.iter().copied().collect::<Vec<_>>());

        clock.advance_a_second();

        assert_eq!(*buckets.current_mut(), 0);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(3),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(2),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
        assert_eq!(vec![0, 3, 2], buckets.iter().copied().collect::<Vec<_>>());

        clock.advance_a_second();

        assert_eq!(*buckets.current_mut(), 0);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(3),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
        assert_eq!(vec![0, 0, 3], buckets.iter().copied().collect::<Vec<_>>());

        clock.advance_a_second();

        assert_eq!(*buckets.current_mut(), 0);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
        assert_eq!(vec![0, 0, 0], buckets.iter().copied().collect::<Vec<_>>());
    }

    #[test]
    fn with_three_uninitialized_items_tick_one_second() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(3, clock.clone());

        assert_eq!(*buckets.current_mut(), 0);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
        assert_eq!(vec![0, 0, 0], buckets.iter().copied().collect::<Vec<_>>());

        *buckets.current_mut() = 1;
        clock.advance_a_second();

        assert_eq!(*buckets.current_mut(), 0);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(1),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
        assert_eq!(vec![0, 1, 0], buckets.iter().copied().collect::<Vec<_>>());

        *buckets.current_mut() = 2;
        clock.advance_a_second();

        assert_eq!(*buckets.current_mut(), 0);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(2),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(1),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
        assert_eq!(vec![0, 2, 1], buckets.iter().copied().collect::<Vec<_>>());

        *buckets.current_mut() = 3;
        clock.advance_a_second();

        assert_eq!(*buckets.current_mut(), 0);
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(0)).copied(),
            Some(0),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(1)).copied(),
            Some(3),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(2)).copied(),
            Some(2),
        );
        assert_eq!(
            buckets.get_at_mut(clock.seconds_in_the_past(3)).copied(),
            None,
        );
        assert_eq!(vec![0, 3, 2], buckets.iter().copied().collect::<Vec<_>>());
    }

    #[test]
    fn shift_by_tick() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(5, clock.clone());
        *buckets.current_mut() = 1;

        assert_eq!(
            vec![1, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        assert_eq!(
            vec![0, 1, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 1, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 0, 1, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 0, 0, 1],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
    }

    #[test]
    fn shift_by_half_ticks() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(3, clock.clone());
        *buckets.current_mut() = 1;

        assert_eq!(vec![1, 0, 0], buckets.iter().copied().collect::<Vec<_>>());
        clock.advance_millis(500);
        assert_eq!(vec![1, 0, 0], buckets.iter().copied().collect::<Vec<_>>());
        clock.advance_millis(500);
        assert_eq!(vec![0, 1, 0], buckets.iter().copied().collect::<Vec<_>>());
        clock.advance_millis(500);
        assert_eq!(vec![0, 1, 0], buckets.iter().copied().collect::<Vec<_>>());
        clock.advance_millis(500);
        assert_eq!(vec![0, 0, 1], buckets.iter().copied().collect::<Vec<_>>());
        clock.advance_millis(500);
        assert_eq!(vec![0, 0, 1], buckets.iter().copied().collect::<Vec<_>>());
        clock.advance_millis(500);
        assert_eq!(vec![0, 0, 0], buckets.iter().copied().collect::<Vec<_>>());
    }

    #[test]
    fn non_equidistant_advance() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(5, clock.clone());
        *buckets.current_mut() = 1;

        assert_eq!(
            vec![1, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );

        clock.advance_millis(1_500);
        assert_eq!(
            vec![0, 1, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );

        clock.advance_millis(500);
        assert_eq!(
            vec![0, 0, 1, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );

        clock.advance_millis(1_500);
        assert_eq!(
            vec![0, 0, 0, 1, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );

        clock.advance_millis(500);
        assert_eq!(
            vec![0, 0, 0, 0, 1],
            buckets.iter().copied().collect::<Vec<_>>()
        );
    }

    #[test]
    fn shift_by_double_tick() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(5, clock.clone());
        *buckets.current_mut() = 1;

        assert_eq!(
            vec![1, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 1, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 0, 0, 1],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
    }

    #[test]
    fn empty_by_over_tick_1() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(5, clock.clone());
        *buckets.current_mut() = 1;

        assert_eq!(
            vec![1, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        clock.advance_a_second();
        clock.advance_a_second();
        clock.advance_a_second();
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
    }

    #[test]
    fn empty_by_over_tick_2() {
        let clock = ManualOffsetClock::default();
        let mut buckets = SecondsBuckets::<u32, _>::with_clock(5, clock.clone());
        *buckets.current_mut() = 1;

        assert_eq!(
            vec![1, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
        clock.advance_a_second();
        clock.advance_a_second();
        clock.advance_a_second();
        clock.advance_a_second();
        clock.advance_a_second();
        clock.advance_a_second();
        assert_eq!(
            vec![0, 0, 0, 0, 0],
            buckets.iter().copied().collect::<Vec<_>>()
        );
    }
}
