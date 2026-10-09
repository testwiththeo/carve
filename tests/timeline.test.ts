import { describe, it, expect, beforeEach } from 'vitest';
import {
  clampElementDuration,
  clampTrackMove,
  clampTrackResize,
  useTimelineStore,
} from '../src/stores/timelineStore';

describe('timeline clamping: clampElementDuration', () => {
  it('clamps element start and end within video trim range', () => {
    const clamped = clampElementDuration(
      { start_ms: 100, end_ms: 5000 },
      1000, // trim_start
      4000  // trim_end
    );
    expect(clamped.start_ms).toBe(1000);
    expect(clamped.end_ms).toBe(4000);
  });

  it('keeps element untouched when already within trim range', () => {
    const clamped = clampElementDuration(
      { start_ms: 1500, end_ms: 3000 },
      1000,
      4000
    );
    expect(clamped.start_ms).toBe(1500);
    expect(clamped.end_ms).toBe(3000);
  });

  it('handles inverted trim handles by normalizing bounds', () => {
    const clamped = clampElementDuration(
      { start_ms: 100, end_ms: 5000 },
      4000, // trim_start > trim_end
      1000  // trim_end
    );
    expect(clamped.start_ms).toBe(1000);
    expect(clamped.end_ms).toBe(4000);
  });

  it('handles zero duration trim window', () => {
    const clamped = clampElementDuration(
      { start_ms: 500, end_ms: 2500 },
      2000,
      2000
    );
    expect(clamped.start_ms).toBe(2000);
    expect(clamped.end_ms).toBe(2000);
  });

  it('clamps element entirely before or after trim range', () => {
    const before = clampElementDuration(
      { start_ms: 0, end_ms: 500 },
      1000,
      3000
    );
    expect(before.start_ms).toBe(1000);
    expect(before.end_ms).toBe(1000);

    const after = clampElementDuration(
      { start_ms: 3500, end_ms: 4500 },
      1000,
      3000
    );
    expect(after.start_ms).toBe(3000);
    expect(after.end_ms).toBe(3000);
  });

  it('handles inverted element duration (start > end)', () => {
    const clamped = clampElementDuration(
      { start_ms: 3500, end_ms: 1500 },
      1000,
      4000
    );
    expect(clamped.start_ms).toBe(1500);
    expect(clamped.end_ms).toBe(3500);
  });
});

describe('timeline clamping: track move and resize helpers', () => {
  it('clamps track move within [0, maxDuration]', () => {
    // 1000ms track trying to move before 0
    expect(clampTrackMove(-200, 1000, 5000)).toEqual({ start_ms: 0, end_ms: 1000 });
    // 1000ms track trying to move past 5000
    expect(clampTrackMove(4500, 1000, 5000)).toEqual({ start_ms: 4000, end_ms: 5000 });
    // Normal move within bounds
    expect(clampTrackMove(2000, 1000, 5000)).toEqual({ start_ms: 2000, end_ms: 3000 });
  });

  it('clamps track resize with min duration and max bounds', () => {
    // Normal resize
    expect(clampTrackResize(1000, 3000, 100, 5000)).toEqual({ start_ms: 1000, end_ms: 3000 });
    // Start dragged past end
    expect(clampTrackResize(3000, 2000, 100, 5000)).toEqual({ start_ms: 1900, end_ms: 2000 });
    // End dragged beyond maxDuration
    expect(clampTrackResize(4000, 6000, 100, 5000)).toEqual({ start_ms: 4000, end_ms: 5000 });
    // Start dragged before 0
    expect(clampTrackResize(-500, 2000, 100, 5000)).toEqual({ start_ms: 0, end_ms: 2000 });
    // End handle dragged left of start + minDurationMs
    expect(clampTrackResize(1000, 500, 100, 5000, 'end')).toEqual({ start_ms: 1000, end_ms: 1100 });
    expect(clampTrackResize(1000, 1050, 100, 5000, 'end')).toEqual({ start_ms: 1000, end_ms: 1100 });
    expect(clampTrackResize(1000, 2000, 100, 5000, 'end')).toEqual({ start_ms: 1000, end_ms: 2000 });
  });
});

describe('useTimelineStore', () => {
  beforeEach(() => {
    useTimelineStore.getState().reset();
  });

  it('has sensible initial defaults', () => {
    const state = useTimelineStore.getState();
    expect(state.durationMs).toBe(5000);
    expect(state.currentTimeMs).toBe(0);
    expect(state.trimStartMs).toBe(0);
    expect(state.trimEndMs).toBe(5000);
    expect(state.isPlaying).toBe(false);
    expect(state.fps).toBe(30);
    expect(state.zoom).toBe(1);
    expect(state.isLooping).toBe(true);
  });

  it('supports play, pause, togglePlay', () => {
    const store = useTimelineStore.getState();
    expect(store.isPlaying).toBe(false);

    store.play();
    expect(useTimelineStore.getState().isPlaying).toBe(true);

    store.pause();
    expect(useTimelineStore.getState().isPlaying).toBe(false);

    store.togglePlay();
    expect(useTimelineStore.getState().isPlaying).toBe(true);
  });

  it('play() rewinds to trimStartMs when at or beyond trimEndMs', () => {
    const store = useTimelineStore.getState();
    store.setTrim(1000, 4000);
    store.seek(4000);
    expect(useTimelineStore.getState().currentTimeMs).toBe(4000);

    store.play();
    expect(useTimelineStore.getState().isPlaying).toBe(true);
    expect(useTimelineStore.getState().currentTimeMs).toBe(1000);
  });

  it('playhead scrubbing via seek clamps between 0 and durationMs', () => {
    const store = useTimelineStore.getState();
    store.seek(2500);
    expect(useTimelineStore.getState().currentTimeMs).toBe(2500);

    store.seek(-500);
    expect(useTimelineStore.getState().currentTimeMs).toBe(0);

    store.seek(99999);
    expect(useTimelineStore.getState().currentTimeMs).toBe(5000);
  });

  it('setTrim clamps and normalizes inverted bounds', () => {
    const store = useTimelineStore.getState();
    store.setTrim(4000, 1000);
    expect(useTimelineStore.getState().trimStartMs).toBe(1000);
    expect(useTimelineStore.getState().trimEndMs).toBe(4000);

    // Clamps to [0, durationMs]
    store.setTrim(-500, 8000);
    expect(useTimelineStore.getState().trimStartMs).toBe(0);
    expect(useTimelineStore.getState().trimEndMs).toBe(5000);
  });

  it('setDuration updates duration and adjusts trim/playhead', () => {
    const store = useTimelineStore.getState();
    store.seek(4500);
    store.setTrim(1000, 5000);

    store.setDuration(3000);
    expect(useTimelineStore.getState().durationMs).toBe(3000);
    expect(useTimelineStore.getState().currentTimeMs).toBe(3000);
    expect(useTimelineStore.getState().trimEndMs).toBe(3000);
  });

  it('updates zoom, fps, isLooping', () => {
    const store = useTimelineStore.getState();
    store.setZoom(2.5);
    expect(useTimelineStore.getState().zoom).toBe(2.5);

    store.setFps(60);
    expect(useTimelineStore.getState().fps).toBe(60);

    store.setLooping(false);
    expect(useTimelineStore.getState().isLooping).toBe(false);
  });
});

describe('timeline and studio component exports', () => {
  it('exports TrackItem, TimelineScrubber, and StudioView', async () => {
    const { TrackItem } = await import('../src/components/timeline/TrackItem');
    const { TimelineScrubber } = await import('../src/components/timeline/TimelineScrubber');
    const { StudioView } = await import('../src/windows/StudioView');

    expect(TrackItem).toBeDefined();
    expect(TimelineScrubber).toBeDefined();
    expect(StudioView).toBeDefined();
  });
});
