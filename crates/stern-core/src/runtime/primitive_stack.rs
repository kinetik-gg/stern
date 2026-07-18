use crate::Transform;
use crate::render::{ClipId, LayerId, Primitive};

use super::types::FrameWarning;

pub(super) fn validate_primitive_stack(primitives: &[Primitive]) -> Vec<FrameWarning> {
    let mut warnings = Vec::new();
    let mut scopes = Vec::new();

    for primitive in primitives {
        match primitive {
            Primitive::ClipBegin { id, .. } => scopes.push(PrimitiveScope::Clip(*id)),
            Primitive::ClipEnd { id } => match scopes.last().copied() {
                Some(PrimitiveScope::Clip(open_id)) if open_id == *id => {
                    scopes.pop();
                }
                _ => warnings.push(FrameWarning::UnmatchedClipEnd { id: *id }),
            },
            Primitive::LayerBegin { id } => scopes.push(PrimitiveScope::Layer(*id)),
            Primitive::LayerEnd { id } => match scopes.last().copied() {
                Some(PrimitiveScope::Layer(open_id)) if open_id == *id => {
                    scopes.pop();
                }
                _ => warnings.push(FrameWarning::UnmatchedLayerEnd { id: *id }),
            },
            Primitive::TransformBegin(Transform { .. }) => {
                scopes.push(PrimitiveScope::Transform);
            }
            Primitive::TransformEnd => match scopes.last().copied() {
                Some(PrimitiveScope::Transform) => {
                    scopes.pop();
                }
                _ => warnings.push(FrameWarning::UnmatchedTransformEnd),
            },
            Primitive::Rect(_)
            | Primitive::Line(_)
            | Primitive::Shadow(_)
            | Primitive::Path(_)
            | Primitive::Icon(_)
            | Primitive::Text(_)
            | Primitive::Image(_)
            | Primitive::Texture(_) => {}
        }
    }

    let mut clips = Vec::new();
    let mut layers = Vec::new();
    let mut transform_depth = 0;
    for scope in scopes {
        match scope {
            PrimitiveScope::Clip(id) => clips.push(id),
            PrimitiveScope::Layer(id) => layers.push(id),
            PrimitiveScope::Transform => transform_depth += 1,
        }
    }

    warnings.extend(
        clips
            .into_iter()
            .rev()
            .map(|id| FrameWarning::UnclosedClip { id }),
    );
    warnings.extend(
        layers
            .into_iter()
            .rev()
            .map(|id| FrameWarning::UnclosedLayer { id }),
    );
    if transform_depth > 0 {
        warnings.push(FrameWarning::UnclosedTransforms {
            count: transform_depth,
        });
    }

    warnings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrimitiveScope {
    Clip(ClipId),
    Layer(LayerId),
    Transform,
}
