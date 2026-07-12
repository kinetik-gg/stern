use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender, TrySendError},
};

use vello::wgpu::{self, Device, Queue};

use crate::{PresenterGpuError, PresenterGpuErrorKind, VelloPresenterError};

const DEVICE_EVENT_CAPACITY: usize = 32;
static NEXT_PRESENTER_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque identity of one presenter's current device generation.
///
/// Values are created only by [`crate::VelloWindowPresenter`]. Cloning a scope
/// is the supported way to retain it for later native-resource operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresenterDeviceScope {
    presenter_id: u64,
    generation: u64,
}

/// Borrowed access to the exact current presenter device and queue.
pub struct PresenterDevice<'a> {
    scope: PresenterDeviceScope,
    device: &'a Device,
    queue: &'a Queue,
}

impl<'a> PresenterDevice<'a> {
    pub(crate) fn new(scope: PresenterDeviceScope, device: &'a Device, queue: &'a Queue) -> Self {
        Self {
            scope,
            device,
            queue,
        }
    }

    /// Returns the validated scope paired with these native handles.
    #[must_use]
    pub const fn scope(&self) -> &PresenterDeviceScope {
        &self.scope
    }

    /// Returns the exact current wgpu device.
    #[must_use]
    pub const fn device(&self) -> &'a Device {
        self.device
    }

    /// Returns the exact current wgpu queue.
    #[must_use]
    pub const fn queue(&self) -> &'a Queue {
        self.queue
    }
}

#[derive(Debug)]
pub(crate) struct DeviceAuthority {
    presenter_id: u64,
    generation: u64,
    current: bool,
}

impl DeviceAuthority {
    pub(crate) fn new() -> Result<Self, VelloPresenterError> {
        let presenter_id = NEXT_PRESENTER_ID
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| VelloPresenterError::GenerationExhausted)?;
        Ok(Self {
            presenter_id,
            generation: 1,
            current: false,
        })
    }

    #[cfg(test)]
    pub(crate) const fn for_test(presenter_id: u64, generation: u64, current: bool) -> Self {
        Self {
            presenter_id,
            generation,
            current,
        }
    }

    pub(crate) fn activate(&mut self) -> PresenterDeviceScope {
        self.current = true;
        self.scope().expect("scope was activated")
    }

    pub(crate) fn scope(&self) -> Option<PresenterDeviceScope> {
        self.current.then_some(PresenterDeviceScope {
            presenter_id: self.presenter_id,
            generation: self.generation,
        })
    }

    pub(crate) fn validate(&self, scope: &PresenterDeviceScope) -> Result<(), VelloPresenterError> {
        if scope.presenter_id != self.presenter_id {
            return Err(VelloPresenterError::ForeignPresenterScope);
        }
        if !self.current || scope.generation != self.generation {
            return Err(VelloPresenterError::StaleDeviceScope);
        }
        Ok(())
    }

    pub(crate) fn invalidate(&mut self) -> Result<bool, VelloPresenterError> {
        if !self.current {
            return Ok(false);
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(VelloPresenterError::GenerationExhausted)?;
        self.current = false;
        Ok(true)
    }

    pub(crate) fn replace(&mut self) -> Result<PresenterDeviceScope, VelloPresenterError> {
        if self.current {
            self.invalidate()?;
        }
        Ok(self.activate())
    }
}

#[derive(Debug, Clone)]
pub(crate) enum DeviceEvent {
    #[cfg(test)]
    Lost {
        scope: PresenterDeviceScope,
        reason: wgpu::DeviceLostReason,
        message: String,
    },
    Error {
        scope: PresenterDeviceScope,
        error: PresenterGpuError,
    },
}

#[derive(Clone)]
pub(crate) struct DeviceEventSender {
    sender: SyncSender<DeviceEvent>,
    overflow: Arc<AtomicU64>,
    loss_pending: Arc<AtomicBool>,
}

impl DeviceEventSender {
    /// Records current-generation device loss outside the bounded error queue.
    ///
    /// A device-loss callback is normally one-shot, so it must remain visible
    /// even when uncaptured errors have saturated their bounded inbox.
    pub(crate) fn signal_loss(&self) {
        self.loss_pending.store(true, Ordering::Release);
    }

    pub(crate) fn send(&self, event: DeviceEvent) {
        match self.sender.try_send(event) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => {
                let _ = self
                    .overflow
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                        Some(value.saturating_add(1))
                    });
            }
        }
    }
}

pub(crate) struct DeviceInbox {
    scope: PresenterDeviceScope,
    receiver: Receiver<DeviceEvent>,
    overflow: Arc<AtomicU64>,
    loss_pending: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
pub(crate) struct CurrentDeviceEvents {
    pub(crate) lost: bool,
    pub(crate) error: Option<PresenterGpuError>,
    pub(crate) overflow: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CurrentDeviceEventOutcome {
    None,
    Lost,
    Actionable(VelloPresenterError),
}

pub(crate) fn classify_current_device_events(
    events: CurrentDeviceEvents,
) -> CurrentDeviceEventOutcome {
    if events.lost {
        CurrentDeviceEventOutcome::Lost
    } else if events.overflow > 0 {
        CurrentDeviceEventOutcome::Actionable(VelloPresenterError::UncapturedErrorOverflow {
            dropped: events.overflow,
        })
    } else if let Some(error) = events.error {
        CurrentDeviceEventOutcome::Actionable(VelloPresenterError::UncapturedGpu(error))
    } else {
        CurrentDeviceEventOutcome::None
    }
}

impl DeviceInbox {
    fn channel(scope: PresenterDeviceScope) -> (Self, DeviceEventSender) {
        let (sender, receiver) = mpsc::sync_channel(DEVICE_EVENT_CAPACITY);
        let overflow = Arc::new(AtomicU64::new(0));
        let loss_pending = Arc::new(AtomicBool::new(false));
        (
            Self {
                scope,
                receiver,
                overflow: Arc::clone(&overflow),
                loss_pending: Arc::clone(&loss_pending),
            },
            DeviceEventSender {
                sender,
                overflow,
                loss_pending,
            },
        )
    }

    pub(crate) fn install(device: &Device, scope: PresenterDeviceScope) -> Self {
        let (inbox, sender) = Self::channel(scope.clone());
        let lost_sender = sender.clone();
        device.set_device_lost_callback(move |_reason, _message| {
            lost_sender.signal_loss();
        });
        device.on_uncaptured_error(Arc::new(move |error| {
            let kind = match &error {
                wgpu::Error::OutOfMemory { .. } => PresenterGpuErrorKind::OutOfMemory,
                wgpu::Error::Validation { .. } => PresenterGpuErrorKind::Validation,
                wgpu::Error::Internal { .. } => PresenterGpuErrorKind::Internal,
            };
            sender.send(DeviceEvent::Error {
                scope: scope.clone(),
                error: PresenterGpuError::new(kind, error.to_string()),
            });
        }));
        inbox
    }

    #[cfg(test)]
    pub(crate) fn for_test(scope: PresenterDeviceScope) -> (Self, DeviceEventSender) {
        Self::channel(scope)
    }

    pub(crate) fn drain(&self) -> (Vec<DeviceEvent>, u64) {
        let mut events = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        let overflow = self.overflow.swap(0, Ordering::AcqRel);
        (events, overflow)
    }

    pub(crate) fn drain_current(&self, current: &PresenterDeviceScope) -> CurrentDeviceEvents {
        let (events, overflow) = self.drain();
        let loss_pending = self.loss_pending.swap(false, Ordering::AcqRel);
        let mut current_events = CurrentDeviceEvents {
            lost: loss_pending && self.scope == *current,
            overflow,
            ..CurrentDeviceEvents::default()
        };
        for event in events {
            match event {
                #[cfg(test)]
                DeviceEvent::Lost {
                    scope,
                    reason,
                    message,
                } if scope == *current => {
                    let _ = (reason, message);
                    current_events.lost = true;
                }
                DeviceEvent::Error { scope, error } if scope == *current => {
                    current_events.error.get_or_insert(error);
                }
                #[cfg(test)]
                DeviceEvent::Lost { .. } => {}
                DeviceEvent::Error { .. } => {}
            }
        }
        current_events
    }
}
