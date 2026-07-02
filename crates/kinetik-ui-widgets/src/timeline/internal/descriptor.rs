#[allow(clippy::wildcard_imports)]
use super::*;

pub(crate) fn validate_timeline_descriptor(
    descriptor: &TimelineDescriptor,
) -> Result<(), TimelineDescriptorError> {
    let mut lane_ids = BTreeSet::new();
    for lane in &descriptor.lanes {
        if !lane_ids.insert(lane.id) {
            return Err(TimelineDescriptorError::DuplicateLaneId { id: lane.id });
        }
    }

    let mut item_ids = BTreeSet::new();
    for item in &descriptor.items {
        if !item_ids.insert(item.id) {
            return Err(TimelineDescriptorError::DuplicateItemId { id: item.id });
        }
    }

    let mut marker_ids = BTreeSet::new();
    for marker in &descriptor.markers {
        if !marker_ids.insert(marker.id) {
            return Err(TimelineDescriptorError::DuplicateMarkerId { id: marker.id });
        }
    }

    let mut keyframe_ids = BTreeSet::new();
    for keyframe in &descriptor.keyframes {
        if !keyframe_ids.insert(keyframe.id) {
            return Err(TimelineDescriptorError::DuplicateKeyframeId { id: keyframe.id });
        }
    }

    for item in &descriptor.items {
        if !lane_ids.contains(&item.lane) {
            return Err(TimelineDescriptorError::UnknownItemLane {
                item: item.id,
                lane: item.lane,
            });
        }
    }

    for keyframe in &descriptor.keyframes {
        if !item_ids.contains(&keyframe.item) {
            return Err(TimelineDescriptorError::UnknownKeyframeItem {
                keyframe: keyframe.id,
                item: keyframe.item,
            });
        }
    }

    Ok(())
}
