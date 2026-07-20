use super::super::{
    Axis, DockPathElement, FrameId, Rect, Vec2, split_child_rects, split_child_rects_with_gutter,
    split_ratio_from_drag, split_ratio_from_drag_with_gutter,
};
use super::{DockNode, DockSplitInsertion, Frame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DockSplitTopologyFingerprint {
    Frame(FrameId),
    Split {
        axis: Axis,
        min_first: u32,
        min_second: u32,
        first: Box<Self>,
        second: Box<Self>,
    },
}

pub(super) fn collect_frames<'a>(node: &'a DockNode, frames: &mut Vec<&'a Frame>) {
    match node {
        DockNode::Frame(frame) => frames.push(frame),
        DockNode::Split { first, second, .. } => {
            collect_frames(first, frames);
            collect_frames(second, frames);
        }
    }
}

pub(crate) fn collect_frame_ids(node: &DockNode) -> Vec<FrameId> {
    let mut frames = Vec::new();
    collect_frame_ids_inner(node, &mut frames);
    frames
}

fn collect_frame_ids_inner(node: &DockNode, frames: &mut Vec<FrameId>) {
    match node {
        DockNode::Frame(frame) => frames.push(frame.id),
        DockNode::Split { first, second, .. } => {
            collect_frame_ids_inner(first, frames);
            collect_frame_ids_inner(second, frames);
        }
    }
}

pub(crate) fn split_children_at_path<'a>(
    node: &'a DockNode,
    path: &[DockPathElement],
) -> Option<(Axis, &'a DockNode, &'a DockNode)> {
    match (node, path) {
        (
            DockNode::Split {
                axis,
                first,
                second,
                ..
            },
            [],
        ) => Some((*axis, first, second)),
        (DockNode::Split { first, .. }, [DockPathElement::First, rest @ ..]) => {
            split_children_at_path(first, rest)
        }
        (DockNode::Split { second, .. }, [DockPathElement::Second, rest @ ..]) => {
            split_children_at_path(second, rest)
        }
        _ => None,
    }
}

pub(super) fn find_frame_mut(node: &mut DockNode, id: FrameId) -> Option<&mut Frame> {
    match node {
        DockNode::Frame(frame) if frame.id == id => Some(frame),
        DockNode::Frame(_) => None,
        DockNode::Split { first, second, .. } => {
            find_frame_mut(first, id).or_else(|| find_frame_mut(second, id))
        }
    }
}

pub(super) fn find_frame(node: &DockNode, id: FrameId) -> Option<&Frame> {
    match node {
        DockNode::Frame(frame) if frame.id == id => Some(frame),
        DockNode::Frame(_) => None,
        DockNode::Split { first, second, .. } => {
            find_frame(first, id).or_else(|| find_frame(second, id))
        }
    }
}

pub(crate) fn frame_is_valid(frame: &Frame) -> bool {
    !frame.panels.is_empty()
}

pub(super) fn first_valid_frame_id(node: &DockNode) -> Option<FrameId> {
    match node {
        DockNode::Frame(frame) if frame_is_valid(frame) => Some(frame.id),
        DockNode::Frame(_) => None,
        DockNode::Split { first, second, .. } => {
            first_valid_frame_id(first).or_else(|| first_valid_frame_id(second))
        }
    }
}

pub(super) fn prune_empty_frames(node: &mut DockNode) -> bool {
    let original = core::mem::replace(node, empty_dock_node());
    match original {
        DockNode::Frame(frame) => {
            let has_panels = !frame.panels.is_empty();
            *node = DockNode::Frame(frame);
            has_panels
        }
        DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            mut first,
            mut second,
        } => {
            let first_has_panels = prune_empty_frames(&mut first);
            let second_has_panels = prune_empty_frames(&mut second);
            match (first_has_panels, second_has_panels) {
                (true, true) => {
                    *node = DockNode::Split {
                        axis,
                        ratio,
                        min_first,
                        min_second,
                        first,
                        second,
                    };
                    true
                }
                (true, false) => {
                    *node = *first;
                    true
                }
                (false, true) => {
                    *node = *second;
                    true
                }
                (false, false) => {
                    *node = DockNode::Split {
                        axis,
                        ratio,
                        min_first,
                        min_second,
                        first,
                        second,
                    };
                    false
                }
            }
        }
    }
}

pub(super) fn insert_frame_split(
    node: &mut DockNode,
    insertion: DockSplitInsertion,
    inserted: Frame,
) -> bool {
    let mut inserted = Some(inserted);
    insert_frame_split_inner(node, insertion, &mut inserted)
}

fn insert_frame_split_inner(
    node: &mut DockNode,
    insertion: DockSplitInsertion,
    inserted: &mut Option<Frame>,
) -> bool {
    match node {
        DockNode::Frame(frame) if frame.id == insertion.target_frame => {
            let Some(inserted) = inserted.take() else {
                return false;
            };
            let target = core::mem::replace(node, empty_dock_node());
            let inserted = DockNode::Frame(inserted);
            let (first, second) = if insertion.placement.insert_before_target() {
                (inserted, target)
            } else {
                (target, inserted)
            };
            *node = DockNode::Split {
                axis: insertion.placement.axis(),
                ratio: insertion.ratio,
                min_first: insertion.min_first,
                min_second: insertion.min_second,
                first: Box::new(first),
                second: Box::new(second),
            };
            true
        }
        DockNode::Frame(_) => false,
        DockNode::Split { first, second, .. } => {
            insert_frame_split_inner(first, insertion, inserted)
                || insert_frame_split_inner(second, insertion, inserted)
        }
    }
}

pub(super) fn swap_frame_leaves(root: &mut DockNode, first: FrameId, second: FrameId) -> bool {
    if first == second {
        return false;
    }

    let Some(first_path) = find_frame_path(root, first) else {
        return false;
    };
    let Some(second_path) = find_frame_path(root, second) else {
        return false;
    };
    if first_path == second_path {
        return false;
    }

    swap_frames_at_paths(root, &first_path, &second_path)
}

fn find_frame_path(root: &DockNode, frame: FrameId) -> Option<Vec<DockPathElement>> {
    let mut path = Vec::new();
    find_frame_path_inner(root, frame, &mut path).then_some(path)
}

fn find_frame_path_inner(node: &DockNode, frame: FrameId, path: &mut Vec<DockPathElement>) -> bool {
    match node {
        DockNode::Frame(candidate) => candidate.id == frame,
        DockNode::Split { first, second, .. } => {
            path.push(DockPathElement::First);
            if find_frame_path_inner(first, frame, path) {
                return true;
            }
            path.pop();

            path.push(DockPathElement::Second);
            if find_frame_path_inner(second, frame, path) {
                return true;
            }
            path.pop();
            false
        }
    }
}

fn frame_at_path_mut<'a>(
    node: &'a mut DockNode,
    path: &[DockPathElement],
) -> Option<&'a mut Frame> {
    match (node, path) {
        (DockNode::Frame(frame), []) => Some(frame),
        (DockNode::Split { first, .. }, [DockPathElement::First, rest @ ..]) => {
            frame_at_path_mut(first, rest)
        }
        (DockNode::Split { second, .. }, [DockPathElement::Second, rest @ ..]) => {
            frame_at_path_mut(second, rest)
        }
        _ => None,
    }
}

fn swap_frames_at_paths(
    node: &mut DockNode,
    first_path: &[DockPathElement],
    second_path: &[DockPathElement],
) -> bool {
    match (first_path, second_path) {
        ([DockPathElement::First, first_rest @ ..], [DockPathElement::First, second_rest @ ..]) => {
            let DockNode::Split { first, .. } = node else {
                return false;
            };
            swap_frames_at_paths(first, first_rest, second_rest)
        }
        (
            [DockPathElement::Second, first_rest @ ..],
            [DockPathElement::Second, second_rest @ ..],
        ) => {
            let DockNode::Split { second, .. } = node else {
                return false;
            };
            swap_frames_at_paths(second, first_rest, second_rest)
        }
        (
            [DockPathElement::First, first_rest @ ..],
            [DockPathElement::Second, second_rest @ ..],
        ) => {
            let DockNode::Split { first, second, .. } = node else {
                return false;
            };
            let Some(first_frame) = frame_at_path_mut(first, first_rest) else {
                return false;
            };
            let Some(second_frame) = frame_at_path_mut(second, second_rest) else {
                return false;
            };
            core::mem::swap(first_frame, second_frame);
            true
        }
        (
            [DockPathElement::Second, first_rest @ ..],
            [DockPathElement::First, second_rest @ ..],
        ) => {
            let DockNode::Split { first, second, .. } = node else {
                return false;
            };
            let Some(first_frame) = frame_at_path_mut(second, first_rest) else {
                return false;
            };
            let Some(second_frame) = frame_at_path_mut(first, second_rest) else {
                return false;
            };
            core::mem::swap(first_frame, second_frame);
            true
        }
        _ => false,
    }
}

fn empty_dock_node() -> DockNode {
    DockNode::Frame(Frame::new(FrameId::from_raw(0), Vec::new()))
}

pub(super) fn resize_split_at_path(
    node: &mut DockNode,
    path: &[DockPathElement],
    bounds: Rect,
    delta: Vec2,
) -> bool {
    let DockNode::Split {
        axis,
        ratio,
        min_first,
        min_second,
        first,
        second,
    } = node
    else {
        return false;
    };

    if path.is_empty() {
        *ratio = split_ratio_from_drag(*axis, bounds, *ratio, *min_first, *min_second, delta);
        return true;
    }

    let (first_rect, second_rect) =
        split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
    match path[0] {
        DockPathElement::First => resize_split_at_path(first, &path[1..], first_rect, delta),
        DockPathElement::Second => resize_split_at_path(second, &path[1..], second_rect, delta),
    }
}

pub(super) fn resize_split_at_path_with_gutter(
    node: &mut DockNode,
    path: &[DockPathElement],
    bounds: Rect,
    delta: Vec2,
    thickness: f32,
) -> bool {
    let DockNode::Split {
        axis,
        ratio,
        min_first,
        min_second,
        first,
        second,
    } = node
    else {
        return false;
    };

    if path.is_empty() {
        *ratio = split_ratio_from_drag_with_gutter(
            *axis,
            bounds,
            *ratio,
            *min_first,
            *min_second,
            thickness,
            delta,
        );
        return true;
    }

    let (first_rect, _, second_rect) =
        split_child_rects_with_gutter(*axis, bounds, *ratio, *min_first, *min_second, thickness);
    match path[0] {
        DockPathElement::First => {
            resize_split_at_path_with_gutter(first, &path[1..], first_rect, delta, thickness)
        }
        DockPathElement::Second => {
            resize_split_at_path_with_gutter(second, &path[1..], second_rect, delta, thickness)
        }
    }
}

pub(super) fn split_ratio_at_path(node: &DockNode, path: &[DockPathElement]) -> Option<f32> {
    let DockNode::Split {
        ratio,
        first,
        second,
        ..
    } = node
    else {
        return None;
    };

    match path {
        [] => Some(*ratio),
        [DockPathElement::First, rest @ ..] => split_ratio_at_path(first, rest),
        [DockPathElement::Second, rest @ ..] => split_ratio_at_path(second, rest),
    }
}

pub(super) fn split_topology_fingerprint_at_path(
    node: &DockNode,
    path: &[DockPathElement],
) -> Option<DockSplitTopologyFingerprint> {
    match (node, path) {
        (DockNode::Split { .. }, []) => Some(topology_fingerprint(node)),
        (DockNode::Split { first, .. }, [DockPathElement::First, rest @ ..]) => {
            split_topology_fingerprint_at_path(first, rest)
        }
        (DockNode::Split { second, .. }, [DockPathElement::Second, rest @ ..]) => {
            split_topology_fingerprint_at_path(second, rest)
        }
        _ => None,
    }
}

fn topology_fingerprint(node: &DockNode) -> DockSplitTopologyFingerprint {
    match node {
        DockNode::Frame(frame) => DockSplitTopologyFingerprint::Frame(frame.id),
        DockNode::Split {
            axis,
            min_first,
            min_second,
            first,
            second,
            ..
        } => DockSplitTopologyFingerprint::Split {
            axis: *axis,
            min_first: min_first.to_bits(),
            min_second: min_second.to_bits(),
            first: Box::new(topology_fingerprint(first)),
            second: Box::new(topology_fingerprint(second)),
        },
    }
}

pub(super) fn set_split_ratio_at_path(
    node: &mut DockNode,
    path: &[DockPathElement],
    restored_ratio: f32,
) -> bool {
    if !restored_ratio.is_finite() || !(0.0..=1.0).contains(&restored_ratio) {
        return false;
    }

    let DockNode::Split {
        ratio,
        first,
        second,
        ..
    } = node
    else {
        return false;
    };

    match path {
        [] => {
            *ratio = restored_ratio;
            true
        }
        [DockPathElement::First, rest @ ..] => set_split_ratio_at_path(first, rest, restored_ratio),
        [DockPathElement::Second, rest @ ..] => {
            set_split_ratio_at_path(second, rest, restored_ratio)
        }
    }
}
