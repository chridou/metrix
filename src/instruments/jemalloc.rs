use jemalloc_ctl::{arenas, epoch, stats};

use crate::snapshot::Snapshot;
use crate::util;

use super::*;

pub struct JemallocStats;

impl PutsSnapshot for JemallocStats {
    fn put_snapshot(&self, into: &mut Snapshot, _descriptive: bool) {
        if let Err(err) = epoch::advance() {
            util::log_error(err);
            return;
        }

        {
            let mut snapshot = Snapshot::default();
            match stats::active::read() {
                Ok(bytes) => snapshot.push("active", bytes),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::allocated::read() {
                Ok(bytes) => snapshot.push("allocated", bytes),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::mapped::read() {
                Ok(bytes) => snapshot.push("mapped", bytes),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::metadata::read() {
                Ok(bytes) => snapshot.push("metadata", bytes),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::resident::read() {
                Ok(bytes) => snapshot.push("resident", bytes),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::retained::read() {
                Ok(bytes) => snapshot.push("retained", bytes),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            into.push("stats", snapshot);
        }

        {
            let mut snapshot = Snapshot::default();
            match arenas::narenas::read() {
                Ok(n) => snapshot.push("narenas", n),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            into.push("arenas", snapshot);
        }

        match jemalloc_ctl::max_background_threads::read() {
            Ok(n) => into.push("max_background_threads", n),
            Err(err) => {
                util::log_error(err);
            }
        };
    }
}
