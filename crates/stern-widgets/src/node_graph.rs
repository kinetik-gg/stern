//! Backend-independent node graph identity, descriptor, and coordinate contracts.

use std::collections::BTreeSet;

use stern_core::{
    Brush, ClipId, Color, CornerRadius, LinePrimitive, Point, Primitive, Rect, RectPrimitive,
    SemanticNode, SemanticRole, SemanticValue, Stroke, TextPrimitive, WidgetId,
};

mod descriptor;
pub use descriptor::*;
mod edges;
pub use edges::*;
mod geometry;
pub use geometry::*;
mod selection;
pub use selection::*;
mod context;
pub use context::*;
mod search;
pub use search::*;
mod links;
pub use links::*;
mod interaction;
pub use interaction::*;
mod hit_test;
pub use hit_test::*;
mod render;
pub use render::*;
mod widget;
pub use widget::*;
mod internal;
#[allow(clippy::wildcard_imports)]
use internal::*;
