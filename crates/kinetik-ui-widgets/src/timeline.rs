//! Data-only timeline ruler, frame-rate, lane, item, and coordinate contracts.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource, Point, Rect,
    SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticValue, WidgetId,
};

mod model;
pub use model::*;
mod transport;
pub use transport::*;
mod layout;
pub use layout::*;
mod interaction;
pub use interaction::*;
mod semantics;
pub use semantics::*;
mod ruler;
pub use ruler::*;
mod internal;
#[allow(clippy::wildcard_imports)]
use internal::*;
