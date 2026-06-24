//! Deterministic observer subscriptions for retained UI targets.

use std::{
    cell::Cell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

use crate::{
    LivenessGeneration, LivenessRegistry, LivenessTargetId, LivenessToken, LivenessUpdateStatus,
};

/// Retained identity for an observer subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObserverSubscriptionId(u64);

impl ObserverSubscriptionId {
    /// Returns the numeric subscription id.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Caller-owned notification identity carried through the observer queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObserverNotificationId(u64);

impl ObserverNotificationId {
    /// Creates a notification id from a caller-owned value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the numeric notification id.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Explicit subscription handle.
///
/// Dropping the handle marks the subscription inactive. The retained registry
/// removes inactive entries on explicit unsubscribe, prune, or drain.
#[derive(Debug)]
pub struct ObserverSubscriptionHandle {
    id: ObserverSubscriptionId,
    active: Rc<Cell<bool>>,
}

impl ObserverSubscriptionHandle {
    fn new(id: ObserverSubscriptionId, active: Rc<Cell<bool>>) -> Self {
        Self { id, active }
    }

    /// Returns the retained subscription id.
    #[must_use]
    pub const fn id(&self) -> ObserverSubscriptionId {
        self.id
    }

    /// Returns true while the handle has not been dropped or explicitly
    /// unsubscribed.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active.get()
    }

    /// Explicitly marks this handle unsubscribed and returns its id.
    #[must_use]
    pub fn unsubscribe(self) -> ObserverSubscriptionId {
        self.active.set(false);
        self.id
    }
}

impl Drop for ObserverSubscriptionHandle {
    fn drop(&mut self) {
        self.active.set(false);
    }
}

#[derive(Debug)]
struct ObserverSubscriptionEntry {
    token: LivenessToken,
    active: Rc<Cell<bool>>,
}

impl ObserverSubscriptionEntry {
    fn new(token: LivenessToken, active: Rc<Cell<bool>>) -> Self {
        Self { token, active }
    }

    fn is_active(&self) -> bool {
        self.active.get()
    }
}

impl Clone for ObserverSubscriptionEntry {
    fn clone(&self) -> Self {
        Self {
            token: self.token,
            active: Rc::new(Cell::new(self.active.get())),
        }
    }
}

impl PartialEq for ObserverSubscriptionEntry {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token && self.active.get() == other.active.get()
    }
}

/// Queued observer notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverNotification {
    subscription_id: ObserverSubscriptionId,
    notification_id: ObserverNotificationId,
}

impl ObserverNotification {
    /// Creates a notification for a retained subscription.
    #[must_use]
    pub const fn new(
        subscription_id: ObserverSubscriptionId,
        notification_id: ObserverNotificationId,
    ) -> Self {
        Self {
            subscription_id,
            notification_id,
        }
    }

    /// Returns the destination subscription id.
    #[must_use]
    pub const fn subscription_id(self) -> ObserverSubscriptionId {
        self.subscription_id
    }

    /// Returns the caller-owned notification id.
    #[must_use]
    pub const fn notification_id(self) -> ObserverNotificationId {
        self.notification_id
    }
}

/// Result of publishing a notification into the retained observer queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverPublishStatus {
    notification: ObserverNotification,
}

impl ObserverPublishStatus {
    fn queued(notification: ObserverNotification) -> Self {
        Self { notification }
    }

    /// Returns the queued notification.
    #[must_use]
    pub const fn notification(self) -> ObserverNotification {
        self.notification
    }
}

/// Live delivery passed to a synchronous observer drain callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverDelivery {
    notification: ObserverNotification,
    token: LivenessToken,
}

impl ObserverDelivery {
    fn new(notification: ObserverNotification, token: LivenessToken) -> Self {
        Self {
            notification,
            token,
        }
    }

    /// Returns the subscription id receiving this delivery.
    #[must_use]
    pub const fn subscription_id(self) -> ObserverSubscriptionId {
        self.notification.subscription_id()
    }

    /// Returns the caller-owned notification id.
    #[must_use]
    pub const fn notification_id(self) -> ObserverNotificationId {
        self.notification.notification_id()
    }

    /// Returns the liveness token validated for this delivery.
    #[must_use]
    pub const fn token(self) -> LivenessToken {
        self.token
    }
}

/// Reason a queued observer notification was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverDeliverySkipReason {
    /// The subscription was explicitly unsubscribed, dropped, or never retained.
    Unsubscribed,
    /// The liveness target was not currently live.
    StaleTarget {
        /// Target carried by the stale subscription token.
        target: LivenessTargetId,
    },
    /// The liveness target was live at a newer generation than the subscription
    /// token.
    StaleGeneration {
        /// Target carried by the stale subscription token.
        target: LivenessTargetId,
        /// Generation carried by the stale subscription token.
        token_generation: LivenessGeneration,
        /// Current retained generation for the target.
        current_generation: LivenessGeneration,
    },
}

impl ObserverDeliverySkipReason {
    fn from_liveness_status(status: LivenessUpdateStatus) -> Self {
        match status {
            LivenessUpdateStatus::Applied => {
                unreachable!("applied liveness status is delivered, not skipped")
            }
            LivenessUpdateStatus::StaleTarget { target } => Self::StaleTarget { target },
            LivenessUpdateStatus::StaleGeneration {
                target,
                token_generation,
                current_generation,
            } => Self::StaleGeneration {
                target,
                token_generation,
                current_generation,
            },
        }
    }
}

/// Skipped observer delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverSkippedDelivery {
    notification: ObserverNotification,
    reason: ObserverDeliverySkipReason,
}

impl ObserverSkippedDelivery {
    fn new(notification: ObserverNotification, reason: ObserverDeliverySkipReason) -> Self {
        Self {
            notification,
            reason,
        }
    }

    /// Returns the subscription id that did not receive this notification.
    #[must_use]
    pub const fn subscription_id(self) -> ObserverSubscriptionId {
        self.notification.subscription_id()
    }

    /// Returns the caller-owned notification id.
    #[must_use]
    pub const fn notification_id(self) -> ObserverNotificationId {
        self.notification.notification_id()
    }

    /// Returns the skipped-delivery reason.
    #[must_use]
    pub const fn reason(self) -> ObserverDeliverySkipReason {
        self.reason
    }
}

/// Observable status for one queued observer notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObserverDeliveryStatus {
    /// The notification was delivered to the drain callback.
    Delivered(ObserverDelivery),
    /// The notification was skipped without running the drain callback.
    Skipped(ObserverSkippedDelivery),
}

impl ObserverDeliveryStatus {
    /// Returns the subscription id for this status.
    #[must_use]
    pub const fn subscription_id(self) -> ObserverSubscriptionId {
        match self {
            Self::Delivered(delivery) => delivery.subscription_id(),
            Self::Skipped(skipped) => skipped.subscription_id(),
        }
    }

    /// Returns the caller-owned notification id for this status.
    #[must_use]
    pub const fn notification_id(self) -> ObserverNotificationId {
        match self {
            Self::Delivered(delivery) => delivery.notification_id(),
            Self::Skipped(skipped) => skipped.notification_id(),
        }
    }
}

/// Result of a synchronous observer drain pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserverDrain {
    statuses: Vec<ObserverDeliveryStatus>,
    reentrant_drain_skipped: bool,
}

impl ObserverDrain {
    fn new(statuses: Vec<ObserverDeliveryStatus>) -> Self {
        Self {
            statuses,
            reentrant_drain_skipped: false,
        }
    }

    fn reentrant_skipped() -> Self {
        Self {
            statuses: Vec::new(),
            reentrant_drain_skipped: true,
        }
    }

    /// Returns the delivery statuses produced by this pass.
    #[must_use]
    pub fn statuses(&self) -> &[ObserverDeliveryStatus] {
        &self.statuses
    }

    /// Consumes the drain result and returns the delivery statuses.
    #[must_use]
    pub fn into_statuses(self) -> Vec<ObserverDeliveryStatus> {
        self.statuses
    }

    /// Returns true when a nested drain call was ignored during an active
    /// drain callback.
    #[must_use]
    pub const fn reentrant_drain_skipped(&self) -> bool {
        self.reentrant_drain_skipped
    }
}

/// Retained observer registry owned by [`crate::UiMemory`].
#[derive(Debug, Default, PartialEq)]
pub struct ObserverRegistry {
    next_subscription_id: u64,
    subscriptions: HashMap<ObserverSubscriptionId, ObserverSubscriptionEntry>,
    queue: VecDeque<ObserverNotification>,
    draining: bool,
}

impl Clone for ObserverRegistry {
    fn clone(&self) -> Self {
        Self {
            next_subscription_id: self.next_subscription_id,
            subscriptions: self.subscriptions.clone(),
            queue: self.queue.clone(),
            draining: self.draining,
        }
    }
}

impl ObserverRegistry {
    /// Creates an empty observer registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when no retained subscriptions or queued notifications
    /// remain.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.subscriptions.is_empty() && self.queue.is_empty()
    }

    /// Returns the number of active retained subscriptions.
    #[must_use]
    pub fn active_subscription_count(&self) -> usize {
        self.subscriptions
            .values()
            .filter(|entry| entry.is_active())
            .count()
    }

    /// Returns the number of queued notifications.
    #[must_use]
    pub fn queued_notification_count(&self) -> usize {
        self.queue.len()
    }

    /// Returns true when the subscription is retained and active.
    #[must_use]
    pub fn is_subscribed(&self, id: ObserverSubscriptionId) -> bool {
        self.subscriptions
            .get(&id)
            .is_some_and(ObserverSubscriptionEntry::is_active)
    }

    /// Retains a subscription tied to the provided liveness token.
    ///
    /// # Panics
    ///
    /// Panics if the subscription id counter overflows.
    pub fn subscribe(&mut self, token: LivenessToken) -> ObserverSubscriptionHandle {
        let id = ObserverSubscriptionId(self.next_subscription_id);
        self.next_subscription_id = self
            .next_subscription_id
            .checked_add(1)
            .expect("observer subscription id overflow");
        let active = Rc::new(Cell::new(true));
        self.subscriptions.insert(
            id,
            ObserverSubscriptionEntry::new(token, Rc::clone(&active)),
        );
        ObserverSubscriptionHandle::new(id, active)
    }

    /// Updates an active subscription to a renewed liveness token.
    ///
    /// Returns false when the subscription is missing or inactive.
    pub fn update_subscription_token(
        &mut self,
        id: ObserverSubscriptionId,
        token: LivenessToken,
    ) -> bool {
        let Some(entry) = self.subscriptions.get_mut(&id) else {
            return false;
        };
        if !entry.is_active() {
            return false;
        }
        entry.token = token;
        true
    }

    /// Explicitly unsubscribes and removes a retained subscription.
    ///
    /// Queued notifications for this id remain observable as skipped during a
    /// later drain pass.
    pub fn unsubscribe(&mut self, id: ObserverSubscriptionId) -> bool {
        let Some(entry) = self.subscriptions.remove(&id) else {
            return false;
        };
        entry.active.set(false);
        true
    }

    /// Removes dropped or explicitly inactive retained subscriptions.
    pub fn prune_inactive_subscriptions(&mut self) {
        self.subscriptions.retain(|_, entry| entry.is_active());
    }

    /// Publishes a notification into the retained FIFO queue.
    pub fn publish(
        &mut self,
        subscription_id: ObserverSubscriptionId,
        notification_id: ObserverNotificationId,
    ) -> ObserverPublishStatus {
        self.enqueue(ObserverNotification::new(subscription_id, notification_id))
    }

    /// Enqueues an observer notification into the retained FIFO queue.
    pub fn enqueue(&mut self, notification: ObserverNotification) -> ObserverPublishStatus {
        self.queue.push_back(notification);
        ObserverPublishStatus::queued(notification)
    }

    /// Drains notifications that were queued at the start of this pass.
    ///
    /// Notifications published by `deliver` are appended to the queue and are
    /// not delivered until a later drain pass.
    pub fn drain(
        &mut self,
        liveness: &LivenessRegistry,
        mut deliver: impl FnMut(&mut ObserverRegistry, ObserverDelivery),
    ) -> ObserverDrain {
        if self.draining {
            return ObserverDrain::reentrant_skipped();
        }

        self.draining = true;
        let pending_at_start = self.queue.len();
        let mut statuses = Vec::with_capacity(pending_at_start);

        for _ in 0..pending_at_start {
            let Some(notification) = self.queue.pop_front() else {
                break;
            };

            match self.delivery_status(liveness, notification) {
                ObserverDeliveryStatus::Delivered(delivery) => {
                    deliver(self, delivery);
                    statuses.push(ObserverDeliveryStatus::Delivered(delivery));
                }
                skipped @ ObserverDeliveryStatus::Skipped(_) => statuses.push(skipped),
            }
        }

        self.draining = false;
        self.prune_inactive_subscriptions();
        ObserverDrain::new(statuses)
    }

    fn delivery_status(
        &mut self,
        liveness: &LivenessRegistry,
        notification: ObserverNotification,
    ) -> ObserverDeliveryStatus {
        let Some(entry) = self.subscriptions.get(&notification.subscription_id) else {
            return ObserverDeliveryStatus::Skipped(ObserverSkippedDelivery::new(
                notification,
                ObserverDeliverySkipReason::Unsubscribed,
            ));
        };
        if !entry.is_active() {
            self.subscriptions.remove(&notification.subscription_id);
            return ObserverDeliveryStatus::Skipped(ObserverSkippedDelivery::new(
                notification,
                ObserverDeliverySkipReason::Unsubscribed,
            ));
        }

        match liveness.validate(entry.token) {
            LivenessUpdateStatus::Applied => {
                ObserverDeliveryStatus::Delivered(ObserverDelivery::new(notification, entry.token))
            }
            status => ObserverDeliveryStatus::Skipped(ObserverSkippedDelivery::new(
                notification,
                ObserverDeliverySkipReason::from_liveness_status(status),
            )),
        }
    }
}
