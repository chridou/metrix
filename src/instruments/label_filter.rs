pub struct LabelFilter<L> {
    internal: LabelFilterInternal<L>,
}

impl<L> LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    pub fn new(label: L) -> Self {
        let mut f = Self::accept_none();
        f.accept_another(label);
        f
    }

    pub fn predicate<P>(p: P) -> Self
    where
        P: Fn(&L) -> bool + Send + 'static,
    {
        Self {
            internal: LabelFilterInternal::predicate(p),
        }
    }

    pub fn accept_all() -> Self {
        Self {
            internal: LabelFilterInternal::AcceptAll,
        }
    }

    pub fn accept_none() -> Self {
        Self {
            internal: LabelFilterInternal::AcceptNone,
        }
    }

    pub fn accept_another(&mut self, label: L) {
        self.internal.add_label(label)
    }

    pub fn accepts(&self, label: &L) -> bool {
        self.internal.accepts(label)
    }

    fn create(internal: LabelFilterInternal<L>) -> Self {
        Self { internal }
    }
}

pub struct AcceptNoLabel;
pub struct AcceptAllLabels;
pub struct AcceptOneLabel<L>(pub L);
pub struct L<L>(pub L);
pub struct LabelPredicate<P>(pub P);
pub struct LP<P>(pub P);

impl<L> From<L> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(v: L) -> Self {
        Self::new(v)
    }
}

impl<L> From<AcceptAllLabels> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(_v: AcceptAllLabels) -> Self {
        Self::accept_all()
    }
}

impl<L> From<AcceptNoLabel> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(_v: AcceptNoLabel) -> Self {
        Self::accept_none()
    }
}

impl<L, P> From<LabelPredicate<P>> for LabelFilter<L>
where
    L: Eq + Send + 'static,
    P: Fn(&L) -> bool + Send + 'static,
{
    fn from(v: LabelPredicate<P>) -> Self {
        Self::predicate(v.0)
    }
}

impl<L, P> From<LP<P>> for LabelFilter<L>
where
    L: Eq + Send + 'static,
    P: Fn(&L) -> bool + Send + 'static,
{
    fn from(v: LP<P>) -> Self {
        Self::predicate(v.0)
    }
}

impl<LL> From<L<LL>> for LabelFilter<LL>
where
    LL: Eq + Send + 'static,
{
    fn from(v: L<LL>) -> Self {
        Self::new(v.0)
    }
}
impl<L> From<AcceptOneLabel<L>> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(v: AcceptOneLabel<L>) -> Self {
        Self::new(v.0)
    }
}

impl<L> From<Vec<L>> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(v: Vec<L>) -> Self {
        let mut f = Self::accept_none();
        for l in v {
            f.accept_another(l)
        }
        f
    }
}

impl<L> From<(L, L)> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(v: (L, L)) -> Self {
        let mut f = Self::new(v.0);
        f.accept_another(v.1);
        f
    }
}

impl<L> From<(L, L, L)> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(v: (L, L, L)) -> Self {
        let mut f = Self::new(v.0);
        f.accept_another(v.1);
        f.accept_another(v.2);
        f
    }
}

impl<L> From<(L, L, L, L)> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(v: (L, L, L, L)) -> Self {
        let mut f = Self::new(v.0);
        f.accept_another(v.1);
        f.accept_another(v.2);
        f.accept_another(v.3);
        f
    }
}

impl<L> From<(L, L, L, L, L)> for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn from(v: (L, L, L, L, L)) -> Self {
        let mut f = Self::new(v.0);
        f.accept_another(v.1);
        f.accept_another(v.2);
        f.accept_another(v.3);
        f.accept_another(v.4);
        f
    }
}

impl<L> Default for LabelFilter<L>
where
    L: Eq + Send + 'static,
{
    fn default() -> Self {
        Self::create(LabelFilterInternal::default())
    }
}

enum LabelFilterInternal<L> {
    AcceptNone,
    AcceptAll,
    One(L),
    Two(L, L),
    Three(L, L, L),
    Four(L, L, L, L),
    Five(L, L, L, L, L),
    Many(Vec<L>),
    Predicate(Box<dyn Fn(&L) -> bool + Send + 'static>),
}

impl<L> LabelFilterInternal<L>
where
    L: Eq + Send + 'static,
{
    pub fn accept_none() -> Self {
        Self::AcceptNone
    }

    pub fn predicate<P>(p: P) -> Self
    where
        P: Fn(&L) -> bool + Send + 'static,
    {
        Self::Predicate(Box::new(p))
    }

    pub fn many(mut labels: Vec<L>) -> Self {
        if labels.is_empty() {
            return LabelFilterInternal::AcceptNone;
        }

        if labels.len() == 1 {
            return LabelFilterInternal::One(labels.pop().unwrap());
        }

        if labels.len() == 2 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            return LabelFilterInternal::Two(b, a);
        }

        if labels.len() == 3 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            return LabelFilterInternal::Three(c, b, a);
        }

        if labels.len() == 4 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            let d = labels.pop().unwrap();
            return LabelFilterInternal::Four(d, c, b, a);
        }

        if labels.len() == 5 {
            let a = labels.pop().unwrap();
            let b = labels.pop().unwrap();
            let c = labels.pop().unwrap();
            let d = labels.pop().unwrap();
            let ee = labels.pop().unwrap();
            return LabelFilterInternal::Five(ee, d, c, b, a);
        }

        LabelFilterInternal::Many(labels)
    }

    pub fn add_label(&mut self, label: L) {
        let old = std::mem::replace(self, LabelFilterInternal::AcceptNone);
        *self = match old {
            LabelFilterInternal::AcceptNone => LabelFilterInternal::One(label),
            LabelFilterInternal::AcceptAll => LabelFilterInternal::AcceptAll,
            LabelFilterInternal::One(a) => LabelFilterInternal::Two(a, label),
            LabelFilterInternal::Two(a, b) => LabelFilterInternal::Three(a, b, label),
            LabelFilterInternal::Three(a, b, c) => LabelFilterInternal::Four(a, b, c, label),
            LabelFilterInternal::Four(a, b, c, d) => LabelFilterInternal::Five(a, b, c, d, label),
            LabelFilterInternal::Five(a, b, c, d, ee) => {
                LabelFilterInternal::Many(vec![a, b, c, d, ee, label])
            }
            LabelFilterInternal::Many(mut many) => {
                many.push(label);
                LabelFilterInternal::Many(many)
            }
            LabelFilterInternal::Predicate(pred) => {
                let new_pred =
                    move |label_to_accept: &L| *label_to_accept == label || pred(label_to_accept);

                LabelFilterInternal::Predicate(Box::new(new_pred))
            }
        }
    }

    pub fn accepts(&self, label: &L) -> bool {
        match self {
            LabelFilterInternal::AcceptNone => false,
            LabelFilterInternal::AcceptAll => true,
            LabelFilterInternal::One(a) => label == a,
            LabelFilterInternal::Two(a, b) => label == a || label == b,
            LabelFilterInternal::Three(a, b, c) => label == a || label == b || label == c,
            LabelFilterInternal::Four(a, b, c, d) => {
                label == a || label == b || label == c || label == d
            }
            LabelFilterInternal::Five(a, b, c, d, ee) => {
                label == a || label == b || label == c || label == d || label == ee
            }
            LabelFilterInternal::Many(many) => many.contains(label),
            LabelFilterInternal::Predicate(ref pred) => pred(label),
        }
    }
}

impl<L> Default for LabelFilterInternal<L> {
    fn default() -> Self {
        Self::AcceptAll
    }
}

#[cfg(test)]
mod test_label_filter {
    use super::*;

    #[test]
    fn empty_filter() {
        let filter = LabelFilter::accept_none();
        assert!(!filter.accepts(&1));
    }

    #[test]
    fn accept_all_filter() {
        let filter = LabelFilter::accept_all();
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
    }

    #[test]
    fn accept_one_filter() {
        let filter: LabelFilter<_> = 1.into();
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(!filter.accepts(&2));
        assert!(!filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_two_filter() {
        let filter: LabelFilter<_> = (1, 2).into();
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(!filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_three_filter() {
        let filter: LabelFilter<_> = (1, 2, 3).into();
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_four_filter() {
        let filter: LabelFilter<_> = (1, 2, 3, 4).into();
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_five_filter() {
        let filter: LabelFilter<_> = (1, 2, 3, 4, 5).into();
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_many_filter() {
        let filter: LabelFilter<_> = vec![1, 2, 3, 4, 5].into();
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn many_filters() {
        let max = 20;
        for n in 1..max {
            let mut labels = Vec::new();
            for i in 1..=n {
                labels.push(i);
            }

            let filter: LabelFilter<_> = labels.into();

            assert!(!filter.accepts(&0));
            assert!(!filter.accepts(&max));

            for i in 1..=n {
                assert!(filter.accepts(&i));
            }
        }
    }
}

#[cfg(test)]
mod test_label_filter_internal {
    use super::*;

    #[test]
    fn empty_filter() {
        let filter = LabelFilterInternal::AcceptNone;
        assert!(!filter.accepts(&1));
    }

    #[test]
    fn accept_all_filter() {
        let filter = LabelFilterInternal::AcceptAll;
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
    }

    #[test]
    fn accept_one_filter() {
        let filter = LabelFilterInternal::One(1);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(!filter.accepts(&2));
        assert!(!filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_two_filter() {
        let filter = LabelFilterInternal::Two(1, 2);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(!filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_three_filter() {
        let filter = LabelFilterInternal::Three(1, 2, 3);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(!filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_four_filter() {
        let filter = LabelFilterInternal::Four(1, 2, 3, 4);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(!filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_five_filter() {
        let filter = LabelFilterInternal::Five(1, 2, 3, 4, 5);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn accept_many_filter() {
        let filter = LabelFilterInternal::Many(vec![1, 2, 3, 4, 5]);
        assert!(!filter.accepts(&0));
        assert!(filter.accepts(&1));
        assert!(filter.accepts(&2));
        assert!(filter.accepts(&3));
        assert!(filter.accepts(&4));
        assert!(filter.accepts(&5));
        assert!(!filter.accepts(&6));
    }

    #[test]
    fn many_filters() {
        let max = 20;
        for n in 1..max {
            let mut labels = Vec::new();
            for i in 1..=n {
                labels.push(i);
            }

            let filter = LabelFilterInternal::many(labels);

            assert!(!filter.accepts(&0));
            assert!(!filter.accepts(&max));

            for i in 1..=n {
                assert!(filter.accepts(&i));
            }
        }
    }
}
