use std::time::{Duration, Instant};

use crossbeam_channel::{
    self, Receiver as CrossbeamReceiver, Sender as CrossbeamSender, TryRecvError,
};

use crate::{
    processor::ProcessesTelemetryMessages, processor::ProcessingOutcome,
    processor::ProcessingStrategy, processor::ProcessorMount, snapshot::Snapshot,
    AggregatesProcessors, PutsSnapshot,
};

#[derive(Clone)]
pub struct AttachedMount {
    pub(crate) sender: CrossbeamSender<ScopedMountMessage>,
}

impl AttachedMount {
    pub fn attached_mount(&mut self, mount: ProcessorMount) -> AttachedMount {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let attached = InternalAttachedMount {
            receiver: Some(receiver),
            inner: mount,
        };

        self.add_processor(attached);

        AttachedMount { sender }
    }

    fn put_processor<P: ProcessesTelemetryMessages>(&self, processor: P) {
        let _ = self
            .sender
            .send(ScopedMountMessage::Processor(Box::new(processor)));
    }

    fn put_snapshooter<S: PutsSnapshot>(&self, snapshooter: S) {
        let _ = self
            .sender
            .send(ScopedMountMessage::Snapshooter(Box::new(snapshooter)));
    }
}

impl AggregatesProcessors for AttachedMount {
    fn add_processor<P: ProcessesTelemetryMessages>(&mut self, processor: P) {
        let _ = self
            .sender
            .send(ScopedMountMessage::Processor(Box::new(processor)));
    }

    fn add_snapshooter<S: PutsSnapshot>(&mut self, snapshooter: S) {
        let _ = self
            .sender
            .send(ScopedMountMessage::Snapshooter(Box::new(snapshooter)));
    }
}

pub(crate) enum ScopedMountMessage {
    Processor(Box<dyn ProcessesTelemetryMessages>),
    Snapshooter(Box<dyn PutsSnapshot>),
}

pub(crate) struct InternalAttachedMount {
    pub receiver: Option<CrossbeamReceiver<ScopedMountMessage>>,
    pub inner: ProcessorMount,
}

impl PutsSnapshot for InternalAttachedMount {
    fn put_snapshot(&self, into: &mut Snapshot, descriptive: bool) {
        self.inner.put_snapshot(into, descriptive);
    }
}

impl ProcessesTelemetryMessages for InternalAttachedMount {
    fn process(&mut self, max: usize, strategy: ProcessingStrategy) -> ProcessingOutcome {
        if let Some(the_receiver) = self.receiver.take() {
            let still_connected = loop {
                match the_receiver.try_recv() {
                    Ok(ScopedMountMessage::Processor(p)) => {
                        self.inner.add_processor_dyn(p);
                    }
                    Ok(ScopedMountMessage::Snapshooter(s)) => {
                        self.inner.add_snapshooter_dyn(s);
                    }
                    Err(TryRecvError::Empty) => {
                        break true;
                    }
                    Err(TryRecvError::Disconnected) => break false,
                }
            };

            if still_connected {
                self.receiver = Some(the_receiver);
            }
        }

        self.inner.process(max, strategy)
    }
}
