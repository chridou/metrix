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
                Ok(bytes) => snapshot.push("active", bytes.into()),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::allocated::read() {
                Ok(bytes) => snapshot.push("allocated", bytes.into()),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::mapped::read() {
                Ok(bytes) => snapshot.push("mapped", bytes.into()),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::metadata::read() {
                Ok(bytes) => snapshot.push("metadata", bytes.into()),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::resident::read() {
                Ok(bytes) => snapshot.push("resident", bytes.into()),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            match stats::retained::read() {
                Ok(bytes) => snapshot.push("retained", bytes.into()),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            into.push("stats", snapshot.into());
        }

        {
            let mut snapshot = Snapshot::default();
            match arenas::narenas::read() {
                Ok(n) => snapshot.push("narenas", n.into()),
                Err(err) => {
                    util::log_error(err);
                    return;
                }
            };
            into.push("arenas", snapshot.into());
        }

        match jemalloc_ctl::max_background_threads::read() {
            Ok(n) => into.push("max_background_threads", n.into()),
            Err(err) => {
                util::log_error(err);
                return;
            }
        };
    }
}
