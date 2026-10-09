import { create } from 'zustand';
import { useAnnotationStore } from './annotationStore';

export interface TimelineStoreState {
  durationMs: number;
  currentTimeMs: number;
  trimStartMs: number;
  trimEndMs: number;
  isPlaying: boolean;
  fps: number;
  zoom: number;
  isLooping: boolean;

  // Actions
  play: () => void;
  pause: () => void;
  togglePlay: () => void;
  seek: (ms: number) => void;
  setTrim: (start: number, end: number) => void;
  setDuration: (durationMs: number) => void;
  setFps: (fps: number) => void;
  setZoom: (zoom: number) => void;
  setLooping: (looping: boolean) => void;
  stepFrame: (direction?: number) => void;
  reset: () => void;
}

const DEFAULT_DURATION_MS = 5000;
const DEFAULT_FPS = 30;

/**
 * Clamps element start and end timestamps to stay within the video trim window.
 * Handles inverted trim bounds and inverted element timestamps safely.
 */
export function clampElementDuration(
  element: { start_ms: number; end_ms: number },
  trim_start: number,
  trim_end: number
): { start_ms: number; end_ms: number } {
  const minTrim = Math.min(trim_start, trim_end);
  const maxTrim = Math.max(trim_start, trim_end);

  const origStart = Math.min(element.start_ms, element.end_ms);
  const origEnd = Math.max(element.start_ms, element.end_ms);

  const clampedStart = Math.max(minTrim, Math.min(maxTrim, origStart));
  const clampedEnd = Math.max(clampedStart, Math.min(maxTrim, origEnd));

  return {
    start_ms: clampedStart,
    end_ms: clampedEnd,
  };
}

/**
 * Clamps track start/end movement to fit completely within [0, maxDurationMs].
 */
export function clampTrackMove(
  newStartMs: number,
  trackDurationMs: number,
  maxDurationMs: number
): { start_ms: number; end_ms: number } {
  const validDuration = Math.max(0, Math.min(trackDurationMs, maxDurationMs));
  const start = Math.max(0, Math.min(maxDurationMs - validDuration, newStartMs));
  return {
    start_ms: start,
    end_ms: start + validDuration,
  };
}

/**
 * Clamps track resizing (start/end) respecting min duration and [0, maxDurationMs] limits.
 * When dragging right handle ('end'), guarantees end >= start + minDurationMs.
 */
export function clampTrackResize(
  newStartMs: number,
  newEndMs: number,
  minDurationMs: number,
  maxDurationMs: number,
  handle: 'start' | 'end' = 'start'
): { start_ms: number; end_ms: number } {
  const minDur = Math.max(0, minDurationMs);
  const safeMax = Math.max(minDur, maxDurationMs);

  if (handle === 'end') {
    const start = Math.max(0, Math.min(safeMax - minDur, newStartMs));
    const end = Math.max(start + minDur, Math.min(safeMax, newEndMs));
    return {
      start_ms: start,
      end_ms: end,
    };
  }

  const end = Math.max(minDur, Math.min(safeMax, newEndMs));
  const start = Math.max(0, Math.min(end - minDur, newStartMs));

  return {
    start_ms: start,
    end_ms: end,
  };
}

export const useTimelineStore = create<TimelineStoreState>((set, get) => ({
  durationMs: DEFAULT_DURATION_MS,
  currentTimeMs: 0,
  trimStartMs: 0,
  trimEndMs: DEFAULT_DURATION_MS,
  isPlaying: false,
  fps: DEFAULT_FPS,
  zoom: 1,
  isLooping: true,

  play: () => {
    const { currentTimeMs, trimStartMs, trimEndMs } = get();
    if (currentTimeMs >= trimEndMs || currentTimeMs < trimStartMs) {
      set({ currentTimeMs: trimStartMs, isPlaying: true });
      try {
        useAnnotationStore.getState().setActiveTimestampMs(trimStartMs);
      } catch {
        // Annotation store may not be loaded in unit tests
      }
    } else {
      set({ isPlaying: true });
    }
  },

  pause: () => {
    set({ isPlaying: false });
  },

  togglePlay: () => {
    const { isPlaying } = get();
    if (isPlaying) {
      get().pause();
    } else {
      get().play();
    }
  },

  seek: (ms: number) => {
    const { durationMs } = get();
    const clamped = Math.max(0, Math.min(durationMs, ms));
    set({ currentTimeMs: clamped });
    try {
      useAnnotationStore.getState().setActiveTimestampMs(clamped);
    } catch {
      // Annotation store may not be loaded in unit tests
    }
  },

  setTrim: (start: number, end: number) => {
    const { durationMs } = get();
    const minBound = Math.max(0, Math.min(start, end));
    const maxBound = Math.min(durationMs, Math.max(start, end));
    set({
      trimStartMs: minBound,
      trimEndMs: maxBound,
    });
  },

  setDuration: (durationMs: number) => {
    const validDuration = Math.max(0, durationMs);
    set((state) => ({
      durationMs: validDuration,
      currentTimeMs: Math.min(validDuration, state.currentTimeMs),
      trimStartMs: Math.min(validDuration, state.trimStartMs),
      trimEndMs: Math.min(validDuration, state.trimEndMs),
    }));
  },

  setFps: (fps: number) => {
    set({ fps: Math.max(1, fps) });
  },

  setZoom: (zoom: number) => {
    set({ zoom: Math.max(0.2, Math.min(5, zoom)) });
  },

  setLooping: (isLooping: boolean) => {
    set({ isLooping });
  },

  stepFrame: (direction = 1) => {
    const { currentTimeMs, fps } = get();
    const frameDurationMs = 1000 / fps;
    get().seek(Math.round(currentTimeMs + direction * frameDurationMs));
  },

  reset: () => {
    set({
      durationMs: DEFAULT_DURATION_MS,
      currentTimeMs: 0,
      trimStartMs: 0,
      trimEndMs: DEFAULT_DURATION_MS,
      isPlaying: false,
      fps: DEFAULT_FPS,
      zoom: 1,
      isLooping: true,
    });
  },
}));
