#[allow(clippy::wildcard_imports)]
use super::*;

pub(crate) fn sanitize_timeline_snap_metadata(snap: TimelineSnapMetadata) -> TimelineSnapMetadata {
    if snap.source == TimelineSnapSource::None {
        TimelineSnapMetadata::unsnapped(snap.requested_time.sanitized())
    } else {
        TimelineSnapMetadata::snapped(
            snap.requested_time.sanitized(),
            snap.snapped_time.sanitized(),
            snap.source,
            snap.target,
        )
    }
}

pub(crate) fn append_frame_snap_candidates(
    candidates: &mut Vec<TimelineSnapCandidate>,
    request: TimelineSnapCandidateRequest<'_>,
) {
    let range = request.range.sanitized();
    if range.is_empty() || request.max_frame_candidates == 0 {
        return;
    }

    let frame_rate = request.frame_rate.sanitized();
    let start = frame_rate
        .time_to_frame(range.start, TimelineFrameRounding::Ceil)
        .raw();
    let end = frame_rate
        .time_to_frame(range.end, TimelineFrameRounding::Floor)
        .raw();
    if end < start {
        return;
    }

    let mut frame = start;
    let mut emitted = 0_usize;
    while frame <= end && emitted < request.max_frame_candidates {
        candidates.push(TimelineSnapCandidate::new(
            frame_rate.frame_to_time(TimelineFrame::from_raw(frame)),
            TimelineSnapSource::Frame,
            None,
        ));
        emitted = emitted.saturating_add(1);
        let Some(next) = frame.checked_add(1) else {
            break;
        };
        frame = next;
    }
}

pub(crate) fn compare_snap_candidates(
    left: TimelineSnapCandidate,
    right: TimelineSnapCandidate,
) -> std::cmp::Ordering {
    snap_source_rank(left.source)
        .cmp(&snap_source_rank(right.source))
        .then_with(|| left.time.seconds().total_cmp(&right.time.seconds()))
        .then_with(|| left.target.cmp(&right.target))
}

pub(crate) fn snap_source_rank(source: TimelineSnapSource) -> u8 {
    match source {
        TimelineSnapSource::Frame => 0,
        TimelineSnapSource::Playhead => 1,
        TimelineSnapSource::RangeBoundary => 2,
        TimelineSnapSource::ItemBoundary => 3,
        TimelineSnapSource::Marker => 4,
        TimelineSnapSource::Keyframe => 5,
        TimelineSnapSource::None => 6,
    }
}
