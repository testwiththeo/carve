import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  isWaylandEnvironment,
  computeOverlayClickthrough,
  createEphemeralFadeTimer,
  EphemeralAnnotationManager,
  AUTO_FADE_TIMEOUT_MS,
} from '../src/utils/overlayManager';

describe('overlay_mode: Wayland detection', () => {
  it('detects Wayland from XDG_SESSION_TYPE', () => {
    expect(isWaylandEnvironment({ XDG_SESSION_TYPE: 'wayland' })).toBe(true);
    expect(isWaylandEnvironment({ XDG_SESSION_TYPE: 'x11' })).toBe(false);
  });

  it('detects Wayland from WAYLAND_DISPLAY', () => {
    expect(isWaylandEnvironment({ WAYLAND_DISPLAY: 'wayland-0' })).toBe(true);
    expect(isWaylandEnvironment({ WAYLAND_DISPLAY: '' })).toBe(false);
  });

  it('defaults to false when environment is empty or undefined', () => {
    expect(isWaylandEnvironment({})).toBe(false);

    // The no-arg form falls back to the host environment; stub it so this
    // assertion is hermetic on both X11 and Wayland machines.
    vi.stubEnv('XDG_SESSION_TYPE', '');
    vi.stubEnv('WAYLAND_DISPLAY', '');
    try {
      expect(isWaylandEnvironment(undefined)).toBe(false);
      expect(isWaylandEnvironment()).toBe(false);
    } finally {
      vi.unstubAllEnvs();
    }
  });

  it('no-arg form detects Wayland from the host environment', () => {
    vi.stubEnv('WAYLAND_DISPLAY', 'wayland-0');
    try {
      expect(isWaylandEnvironment()).toBe(true);
    } finally {
      vi.unstubAllEnvs();
    }
  });
});

describe('overlay_mode: Ephemeral auto-fade timer (1.5s on Wayland)', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('has exact 1.5s (1500ms) auto fade duration constant', () => {
    expect(AUTO_FADE_TIMEOUT_MS).toBe(1500);
  });

  it('triggers onFade callback exactly after 1500ms', () => {
    const onFade = vi.fn();
    createEphemeralFadeTimer('stroke_123', onFade);

    expect(onFade).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1499);
    expect(onFade).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(onFade).toHaveBeenCalledTimes(1);
    expect(onFade).toHaveBeenCalledWith('stroke_123');
  });

  it('allows cancelling ephemeral fade timer before 1500ms expires', () => {
    const onFade = vi.fn();
    const timer = createEphemeralFadeTimer('stroke_cancel', onFade);

    vi.advanceTimersByTime(800);
    timer.cancel();

    vi.advanceTimersByTime(1000);
    expect(onFade).not.toHaveBeenCalled();
  });

  it('EphemeralAnnotationManager manages multiple ephemeral strokes', () => {
    const manager = new EphemeralAnnotationManager();
    const expiredIds: string[] = [];
    const onExpire = (id: string) => expiredIds.push(id);

    // Add stroke in Wayland mode (enabled = true)
    manager.track('elem_1', onExpire, true);
    expect(manager.hasActiveTimer('elem_1')).toBe(true);

    vi.advanceTimersByTime(1000);
    manager.track('elem_2', onExpire, true);

    // elem_1 expires at 1500ms
    vi.advanceTimersByTime(500);
    expect(expiredIds).toEqual(['elem_1']);
    expect(manager.hasActiveTimer('elem_1')).toBe(false);
    expect(manager.hasActiveTimer('elem_2')).toBe(true);

    // elem_2 expires at 2500ms (1000ms + 1500ms)
    vi.advanceTimersByTime(1000);
    expect(expiredIds).toEqual(['elem_1', 'elem_2']);
    expect(manager.hasActiveTimer('elem_2')).toBe(false);
  });

  it('EphemeralAnnotationManager does not schedule fade when disabled (non-Wayland)', () => {
    const manager = new EphemeralAnnotationManager();
    const onExpire = vi.fn();

    manager.track('elem_persistent', onExpire, false);
    expect(manager.hasActiveTimer('elem_persistent')).toBe(false);

    vi.advanceTimersByTime(5000);
    expect(onExpire).not.toHaveBeenCalled();
  });

  it('EphemeralAnnotationManager cancels all pending timers on clear', () => {
    const manager = new EphemeralAnnotationManager();
    const onExpire = vi.fn();

    manager.track('elem_a', onExpire, true);
    manager.track('elem_b', onExpire, true);
    manager.clear();

    expect(manager.hasActiveTimer('elem_a')).toBe(false);
    expect(manager.hasActiveTimer('elem_b')).toBe(false);

    vi.advanceTimersByTime(3000);
    expect(onExpire).not.toHaveBeenCalled();
  });
});

describe('overlay_mode: Clickthrough modes', () => {
  it('enables clickthrough when manual toggle is ON regardless of tool', () => {
    expect(
      computeOverlayClickthrough({ activeTool: 'arrow', manualClickthrough: true })
    ).toBe(true);
    expect(
      computeOverlayClickthrough({ activeTool: 'pencil', manualClickthrough: true })
    ).toBe(true);
    expect(
      computeOverlayClickthrough({ activeTool: null, manualClickthrough: true })
    ).toBe(true);
  });

  it('enables clickthrough when tool is unselected or null/none', () => {
    expect(
      computeOverlayClickthrough({ activeTool: null, manualClickthrough: false })
    ).toBe(true);
    expect(
      computeOverlayClickthrough({ activeTool: 'none', manualClickthrough: false })
    ).toBe(true);
    expect(
      computeOverlayClickthrough({ activeTool: 'select', manualClickthrough: false })
    ).toBe(true);
  });

  it('disables clickthrough when a drawing tool is active to capture pointer input', () => {
    expect(
      computeOverlayClickthrough({ activeTool: 'arrow', manualClickthrough: false })
    ).toBe(false);
    expect(
      computeOverlayClickthrough({ activeTool: 'rectangle', manualClickthrough: false })
    ).toBe(false);
    expect(
      computeOverlayClickthrough({ activeTool: 'text', manualClickthrough: false })
    ).toBe(false);
    expect(
      computeOverlayClickthrough({ activeTool: 'counter_badge', manualClickthrough: false })
    ).toBe(false);
    expect(
      computeOverlayClickthrough({ activeTool: 'blur', manualClickthrough: false })
    ).toBe(false);
  });
});
