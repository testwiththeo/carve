/**
 * Duration in milliseconds for presenter-mode live strokes to automatically fade out.
 */
export const AUTO_FADE_TIMEOUT_MS = 1500;

export interface OverlayEnvironment {
  XDG_SESSION_TYPE?: string;
  WAYLAND_DISPLAY?: string;
  [key: string]: string | undefined;
}

/**
 * Checks whether the current runtime environment is running under Wayland.
 * Accepts optional explicit environment variables or checks window/process fallback.
 */
export function isWaylandEnvironment(env?: OverlayEnvironment): boolean {
  if (env) {
    if (env.XDG_SESSION_TYPE?.toLowerCase() === 'wayland') return true;
    if (env.WAYLAND_DISPLAY && env.WAYLAND_DISPLAY.length > 0) return true;
    return false;
  }

  // Fallback checks in browser / Node / Tauri webview environment
  if (typeof process !== 'undefined' && process.env) {
    const session = process.env.XDG_SESSION_TYPE?.toLowerCase();
    if (session === 'wayland') return true;
    if (process.env.WAYLAND_DISPLAY) return true;
  }

  return false;
}

export interface ClickthroughOptions {
  activeTool: string | null;
  manualClickthrough: boolean;
}

/**
 * Computes whether pointer events should pass through the overlay window.
 *
 * Rules:
 * - If manual clickthrough is toggled on, clickthrough is active.
 * - If no drawing tool is selected (null, 'none', 'select'), clickthrough is active so the user can interact with underlying windows.
 * - When a drawing tool (e.g. arrow, shape, text) is active, clickthrough is disabled to receive canvas input.
 */
export function computeOverlayClickthrough({
  activeTool,
  manualClickthrough,
}: ClickthroughOptions): boolean {
  if (manualClickthrough) {
    return true;
  }

  if (!activeTool || activeTool === 'none' || activeTool === 'select') {
    return true;
  }

  return false;
}

export interface EphemeralTimerHandle {
  id: string;
  cancel: () => void;
}

/**
 * Creates a cancellation-capable timer that calls `onFade` with the elementId after `durationMs` (default 1500ms).
 */
export function createEphemeralFadeTimer(
  elementId: string,
  onFade: (id: string) => void,
  durationMs: number = AUTO_FADE_TIMEOUT_MS
): EphemeralTimerHandle {
  const timer = setTimeout(() => {
    onFade(elementId);
  }, durationMs);

  return {
    id: elementId,
    cancel: () => clearTimeout(timer),
  };
}

/**
 * Tracks active ephemeral timers for live drawing elements.
 */
export class EphemeralAnnotationManager {
  private activeTimers = new Map<string, EphemeralTimerHandle>();

  /**
   * Tracks an element stroke for auto-fade if `enabled` is true.
   */
  track(
    elementId: string,
    onExpire: (id: string) => void,
    enabled = true,
    durationMs: number = AUTO_FADE_TIMEOUT_MS
  ): void {
    this.cancel(elementId);

    if (!enabled) {
      return;
    }

    const handle = createEphemeralFadeTimer(
      elementId,
      (id) => {
        this.activeTimers.delete(id);
        onExpire(id);
      },
      durationMs
    );

    this.activeTimers.set(elementId, handle);
  }

  cancel(elementId: string): void {
    const existing = this.activeTimers.get(elementId);
    if (existing) {
      existing.cancel();
      this.activeTimers.delete(elementId);
    }
  }

  clear(): void {
    for (const handle of this.activeTimers.values()) {
      handle.cancel();
    }
    this.activeTimers.clear();
  }

  hasActiveTimer(elementId: string): boolean {
    return this.activeTimers.has(elementId);
  }
}
