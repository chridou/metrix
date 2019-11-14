//! Pulling data from the backend for monitoring
use std::fmt;
use std::time::Duration;

use json::{stringify, stringify_pretty, JsonValue};

/// A `Snapshot` which contains measured values
/// at a point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub items: Vec<(String, ItemKind)>,
}

impl Default for Snapshot {
    fn default() -> Snapshot {
        Snapshot { items: Vec::new() }
    }
}

impl Snapshot {
    pub fn push<K: Into<String>>(&mut self, k: K, v: ItemKind) {
        self.items.push((k.into(), v))
    }

    /// Find an item on a path with a given a separator.
    ///
    /// Same as `find` but with a configurable separator.
    pub fn find_with_separator(&self, path: &str, separator: char) -> FindItem {
        let path: Vec<&str> = path.split(separator).filter(|x| !x.is_empty()).collect();

        if path.is_empty() {
            return FindItem::NotFound;
        }

        if let Some(item) = self
            .items
            .iter()
            .find(|&&(ref name, _)| name == path[0])
            .map(|x| &x.1)
        {
            find_item(item, &path[1..])
        } else {
            FindItem::NotFound
        }
    }

    /// Find an item on a path with a `/` as a separator.
    ///
    /// If the path is empty `None` is returned.
    ///
    /// Since a `Snapshot` may contain multiple items with the same name
    /// only the first found will be returned.
    ///
    /// If a prefix of a path leads to a value that value is returned
    /// and the rest of the path is discarded.
    ///
    /// Empty segments of a path are ignored.
    ///
    /// # Example
    ///
    /// ```
    /// use metrix::snapshot::*;
    /// use metrix::snapshot::FindItem::*;
    ///
    /// // a -> 23
    /// // b -> c -> 42
    ///
    /// let inner = ItemKind::Snapshot(Snapshot {
    ///     items: vec![("c".to_string(), ItemKind::UInt(42))],
    /// });
    ///
    /// let snapshot = Snapshot {
    ///     items: vec![
    ///         ("a".to_string(), ItemKind::UInt(23)),
    ///         ("b".to_string(), inner.clone()),
    ///     ],
    /// };
    ///
    /// assert_eq!(snapshot.find("a"), Found(&ItemKind::UInt(23)));
    /// assert_eq!(snapshot.find("a/x"), Found(&ItemKind::UInt(23)));
    /// assert_eq!(snapshot.find("/a/x"), Found(&ItemKind::UInt(23)));
    ///
    /// assert_eq!(snapshot.find("b"), Found(&inner));
    ///
    /// assert_eq!(snapshot.find("b/c"), Found(&ItemKind::UInt(42)));
    /// assert_eq!(snapshot.find("/b//c"), Found(&ItemKind::UInt(42)));
    ///
    /// assert_eq!(snapshot.find("b/c/x"), Found(&ItemKind::UInt(42)));
    ///
    /// assert_eq!(snapshot.find(""), NotFound);
    ///
    /// assert_eq!(snapshot.find("/"), NotFound);
    /// ```
    pub fn find(&self, path: &str) -> FindItem {
        self.find_with_separator(path, '/')
    }

    /// Output JSON with default settings.
    pub fn to_default_json(&self) -> String {
        self.to_json_internal(&JsonConfig::default())
    }

    /// Output JSON with the given settings.
    pub fn to_json(&self, config: &JsonConfig) -> String {
        self.to_json_internal(config)
    }

    fn to_json_internal(&self, config: &JsonConfig) -> String {
        let data = self.to_json_value(config);

        if let Some(indent) = config.pretty {
            stringify_pretty(data, indent)
        } else {
            stringify(data)
        }
    }

    fn to_json_value(&self, config: &JsonConfig) -> JsonValue {
        let mut data = JsonValue::new_object();

        self.items
            .iter()
            .for_each(|&(ref k, ref v)| data[k] = v.to_json_value(config));

        data
    }
}

/// Finds an item in a `Snapshot`
///
/// `path` are the segments of the path.
///
/// Since a `Snapshot` may contain multiple items with the same name
/// only the first found will be returned.
///
/// If a prefix of a path leads to a value that value is returned
/// and the rest of the path is discarded.
///
/// Empty segments of a path are ignored.
///
/// # Example
///
/// ```
/// use metrix::snapshot::*;
/// use metrix::snapshot::FindItem::*;
///
/// // a -> 23
/// // b -> c -> 42
///
/// let inner = ItemKind::Snapshot(Snapshot {
///     items: vec![("c".to_string(), ItemKind::UInt(42))],
/// });
///
/// let snapshot = ItemKind::Snapshot(Snapshot {
///     items: vec![
///         ("a".to_string(), ItemKind::UInt(23)),
///         ("b".to_string(), inner.clone()),
///     ],
/// });
///
/// assert_eq!(find_item(&snapshot, &["a"]), Found(&ItemKind::UInt(23)));
/// assert_eq!(find_item(&snapshot, &["a", "x"]), Found(&ItemKind::UInt(23)));
/// assert_eq!(
///     find_item(&snapshot, &["", "a", "x"]),
///     Found(&ItemKind::UInt(23))
/// );
///
/// assert_eq!(find_item(&snapshot, &["b"]), Found(&inner));
///
/// assert_eq!(find_item(&snapshot, &["b", "c"]), Found(&ItemKind::UInt(42)));
/// assert_eq!(
///     find_item(&snapshot, &["", "b", "", "c"]),
///     Found(&ItemKind::UInt(42))
/// );
///
/// assert_eq!(
///     find_item(&snapshot, &["b", "c", "x"]),
///     Found(&ItemKind::UInt(42))
/// );
///
/// assert_eq!(find_item::<String>(&snapshot, &[]), Found(&snapshot));
///
/// assert_eq!(find_item(&snapshot, &[""]), Found(&snapshot));
/// ```
pub fn find_item<'a, T>(item: &'a ItemKind, path: &[T]) -> FindItem<'a>
where
    T: AsRef<str>,
{
    if path.is_empty() {
        return FindItem::Found(item);
    };

    if path[0].as_ref().is_empty() {
        return find_item(item, &path[1..]);
    }

    match *item {
        ItemKind::Snapshot(ref snapshot) => {
            if let Some(item) = snapshot
                .items
                .iter()
                .find(|&&(ref name, _)| name == path[0].as_ref())
                .map(|x| &x.1)
            {
                find_item(item, &path[1..])
            } else {
                FindItem::NotFound
            }
        }
        ref other => FindItem::Found(other),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FindItem<'a> {
    Found(&'a ItemKind),
    NotFound,
}

impl<'a> FindItem<'a> {
    pub fn opt(&self) -> Option<&ItemKind> {
        use self::FindItem::*;
        match self {
            Found(ref v) => Some(v),
            NotFound => None,
        }
    }

    pub fn find_with_separator(&self, path: &str, separator: char) -> FindItem {
        match self {
            FindItem::Found(ItemKind::Snapshot(snapshot)) => {
                snapshot.find_with_separator(path, separator)
            }
            _ => FindItem::NotFound,
        }
    }

    pub fn find(&self, path: &str) -> FindItem {
        self.find_with_separator(path, '/')
    }

    pub fn to_duration_nanoseconds(&self) -> FindDurationItem {
        match self {
            FindItem::Found(ref v) => match v {
                ItemKind::UInt(v) => FindDurationItem::Duration(Duration::from_nanos(*v)),
                ItemKind::Int(v) => FindDurationItem::Duration(Duration::from_nanos(*v as u64)),
                _ => FindDurationItem::NotADuration,
            },
            FindItem::NotFound => FindDurationItem::NotFound,
        }
    }

    pub fn to_duration_microseconds(&self) -> FindDurationItem {
        match self {
            FindItem::Found(ref v) => match v {
                ItemKind::UInt(v) => FindDurationItem::Duration(Duration::from_micros(*v)),
                ItemKind::Int(v) => FindDurationItem::Duration(Duration::from_micros(*v as u64)),
                _ => FindDurationItem::NotADuration,
            },
            FindItem::NotFound => FindDurationItem::NotFound,
        }
    }

    pub fn to_duration_milliseconds(&self) -> FindDurationItem {
        match self {
            FindItem::Found(ref v) => match v {
                ItemKind::UInt(v) => FindDurationItem::Duration(Duration::from_millis(*v)),
                ItemKind::Int(v) => FindDurationItem::Duration(Duration::from_millis(*v as u64)),
                _ => FindDurationItem::NotADuration,
            },
            FindItem::NotFound => FindDurationItem::NotFound,
        }
    }

    pub fn to_duration_seconds(&self) -> FindDurationItem {
        match self {
            FindItem::Found(ref v) => match v {
                ItemKind::UInt(v) => FindDurationItem::Duration(Duration::from_secs(*v)),
                ItemKind::Int(v) => FindDurationItem::Duration(Duration::from_secs(*v as u64)),
                _ => FindDurationItem::NotADuration,
            },
            FindItem::NotFound => FindDurationItem::NotFound,
        }
    }
}

impl<'a> fmt::Display for FindItem<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::FindItem::*;
        match self {
            Found(ref v) => write!(f, "{}", v),
            NotFound => write!(f, "<item not found>"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindDurationItem {
    NotFound,
    NotADuration,
    Duration(Duration),
}

impl FindDurationItem {
    pub fn opt(self) -> Option<Duration> {
        match self {
            FindDurationItem::Duration(d) => Some(d),
            _ => None,
        }
    }
}

impl fmt::Display for FindDurationItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::FindDurationItem::*;
        match self {
            Duration(d) => write!(f, "{:?}", d),
            NotFound => write!(f, "<item not found>"),
            NotADuration => write!(f, "<item not a duration>"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    /// Simply a `String` in the `Snapshot.
    ///
    /// Unfortunately the name `String` was already taken...
    Text(String),
    Boolean(bool),
    Float(f64),
    UInt(u64),
    Int(i64),
    Snapshot(Snapshot),
}

impl ItemKind {
    fn to_json_value(&self, config: &JsonConfig) -> JsonValue {
        match *self {
            ItemKind::Text(ref v) => v.clone().into(),
            ItemKind::Boolean(v) => {
                if config.make_booleans_ints {
                    if v {
                        1.into()
                    } else {
                        0.into()
                    }
                } else {
                    v.into()
                }
            }
            ItemKind::Float(v) => v.into(),
            ItemKind::UInt(v) => v.into(),
            ItemKind::Int(v) => v.into(),
            ItemKind::Snapshot(ref snapshot) => snapshot.to_json_value(config),
        }
    }
}

impl fmt::Display for ItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::ItemKind::*;
        match self {
            Text(ref v) => write!(f, "{}", v),
            Boolean(v) => write!(f, "{}", v),
            Float(v) => write!(f, "{}", v),
            UInt(v) => write!(f, "{}", v),
            Int(v) => write!(f, "{}", v),
            Snapshot(ref snapshot) => write!(f, "Snapshot({} items)", snapshot.items.len()),
        }
    }
}

pub struct JsonConfig {
    /// Serialize `true` as `1` and `false` as `0`
    pub make_booleans_ints: bool,

    /// Configure pretty JSON output.
    ///
    /// Produce pretty JSON with the given indentation if `Some(indentation)`.
    /// If `None` compact JSON is generated.
    pub pretty: Option<u16>,
}

impl Default for JsonConfig {
    fn default() -> JsonConfig {
        JsonConfig {
            make_booleans_ints: false,
            pretty: None,
        }
    }
}

impl From<u64> for ItemKind {
    fn from(what: u64) -> ItemKind {
        ItemKind::UInt(what)
    }
}

impl From<u32> for ItemKind {
    fn from(what: u32) -> ItemKind {
        ItemKind::UInt(what as u64)
    }
}

impl From<u16> for ItemKind {
    fn from(what: u16) -> ItemKind {
        ItemKind::UInt(what as u64)
    }
}

impl From<u8> for ItemKind {
    fn from(what: u8) -> ItemKind {
        ItemKind::UInt(what as u64)
    }
}

impl From<usize> for ItemKind {
    fn from(what: usize) -> ItemKind {
        ItemKind::UInt(what as u64)
    }
}

impl From<i64> for ItemKind {
    fn from(what: i64) -> ItemKind {
        ItemKind::Int(what)
    }
}

impl From<i32> for ItemKind {
    fn from(what: i32) -> ItemKind {
        ItemKind::Int(what as i64)
    }
}

impl From<i16> for ItemKind {
    fn from(what: i16) -> ItemKind {
        ItemKind::Int(what as i64)
    }
}

impl From<i8> for ItemKind {
    fn from(what: i8) -> ItemKind {
        ItemKind::Int(what as i64)
    }
}

impl From<isize> for ItemKind {
    fn from(what: isize) -> ItemKind {
        ItemKind::Int(what as i64)
    }
}

impl From<String> for ItemKind {
    fn from(what: String) -> ItemKind {
        ItemKind::Text(what)
    }
}

impl<'a> From<&'a str> for ItemKind {
    fn from(what: &'a str) -> ItemKind {
        ItemKind::Text(what.into())
    }
}

impl From<f64> for ItemKind {
    fn from(what: f64) -> ItemKind {
        ItemKind::Float(what)
    }
}

impl From<f32> for ItemKind {
    fn from(what: f32) -> ItemKind {
        ItemKind::Float(what as f64)
    }
}

impl From<bool> for ItemKind {
    fn from(what: bool) -> ItemKind {
        ItemKind::Boolean(what)
    }
}
