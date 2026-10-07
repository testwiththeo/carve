import React, { useRef, useEffect, useCallback, useMemo } from 'react';
import {
  Play,
  Pause,
  RotateCcw,
  SkipBack,
  SkipForward,
  Repeat,
  ZoomIn,
  ZoomOut,
  Scissors,
} from 'lucide-react';
import { useTimelineStore } from '../../stores/timelineStore';
import { useAnnotationStore } from '../../stores/annotationStore';
import { TrackItem } from './TrackItem';

export interface TimelineScrubberProps {
  className?: string;
  onSeek?: (ms: number) => void;
}

export const TimelineScrubber: React.FC<TimelineScrubberProps> = ({
  className = '',
  onSeek,
}) => {
  const {
    durationMs,
    currentTimeMs,
    trimStartMs,
    trimEndMs,
    isPlaying,
    zoom,
    isLooping,
    pause,
    togglePlay,
    seek,
    setTrim,
    setZoom,
    setLooping,
    stepFrame,
  } = useTimelineStore();

  const {
    elements,
    selectedElementId,
    selectElement,
    updateElement,
    removeElement,
    setActiveTimestampMs,
  } = useAnnotationStore();

  const rulerRef = useRef<HTMLDivElement>(null);
  const tracksContainerRef = useRef<HTMLDivElement>(null);
  const animFrameRef = useRef<number | null>(null);
  const lastTimeRef = useRef<number | null>(null);

  // Playback continuous animation loop
  useEffect(() => {
    if (!isPlaying) {
      if (animFrameRef.current) {
        cancelAnimationFrame(animFrameRef.current);
        animFrameRef.current = null;
      }
      lastTimeRef.current = null;
      return;
    }

    const tick = (now: number) => {
      if (lastTimeRef.current === null) {
        lastTimeRef.current = now;
      }
      const deltaMs = now - lastTimeRef.current;
      lastTimeRef.current = now;

      const store = useTimelineStore.getState();
      const nextTime = store.currentTimeMs + deltaMs;

      if (nextTime >= store.trimEndMs) {
        if (store.isLooping) {
          seek(store.trimStartMs);
          setActiveTimestampMs(store.trimStartMs);
          onSeek?.(store.trimStartMs);
        } else {
          seek(store.trimEndMs);
          setActiveTimestampMs(store.trimEndMs);
          onSeek?.(store.trimEndMs);
          pause();
          return;
        }
      } else {
        seek(nextTime);
        setActiveTimestampMs(nextTime);
        onSeek?.(nextTime);
      }

      animFrameRef.current = requestAnimationFrame(tick);
    };

    animFrameRef.current = requestAnimationFrame(tick);

    return () => {
      if (animFrameRef.current) {
        cancelAnimationFrame(animFrameRef.current);
        animFrameRef.current = null;
      }
    };
  }, [isPlaying, isLooping, seek, pause, setActiveTimestampMs, onSeek]);

  // Keyboard shortcut listener: Space = play/pause, Left/Right = frame step
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).tagName === 'INPUT' || (e.target as HTMLElement).tagName === 'TEXTAREA') {
        return;
      }

      if (e.code === 'Space') {
        e.preventDefault();
        togglePlay();
      } else if (e.code === 'ArrowLeft') {
        e.preventDefault();
        stepFrame(-1);
      } else if (e.code === 'ArrowRight') {
        e.preventDefault();
        stepFrame(1);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [togglePlay, stepFrame]);

  // Seeker calculation helper
  const calculateTimestampFromEvent = useCallback(
    (clientX: number, targetElement: HTMLElement | null): number => {
      if (!targetElement) return 0;
      const rect = targetElement.getBoundingClientRect();
      const relativeX = clientX - rect.left;
      const ratio = Math.max(0, Math.min(1, relativeX / rect.width));
      return Math.round(ratio * durationMs);
    },
    [durationMs]
  );

  // Ruler click / drag scrubbing
  const handleRulerMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if ((e.target as HTMLElement).closest('.trim-handle')) return;

      const target = rulerRef.current;
      const targetMs = calculateTimestampFromEvent(e.clientX, target);
      seek(targetMs);
      setActiveTimestampMs(targetMs);
      onSeek?.(targetMs);

      const onMouseMove = (moveEvent: MouseEvent) => {
        const moveMs = calculateTimestampFromEvent(moveEvent.clientX, target);
        seek(moveMs);
        setActiveTimestampMs(moveMs);
        onSeek?.(moveMs);
      };

      const onMouseUp = () => {
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    },
    [calculateTimestampFromEvent, seek, setActiveTimestampMs, onSeek]
  );

  // Left Trim Handle drag
  const handleLeftTrimMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      const target = rulerRef.current;
      if (!target) return;

      const onMouseMove = (moveEvent: MouseEvent) => {
        const moveMs = calculateTimestampFromEvent(moveEvent.clientX, target);
        const safeStart = Math.max(0, Math.min(trimEndMs - 100, moveMs));
        setTrim(safeStart, trimEndMs);
      };

      const onMouseUp = () => {
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    },
    [calculateTimestampFromEvent, setTrim, trimEndMs]
  );

  // Right Trim Handle drag
  const handleRightTrimMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      const target = rulerRef.current;
      if (!target) return;

      const onMouseMove = (moveEvent: MouseEvent) => {
        const moveMs = calculateTimestampFromEvent(moveEvent.clientX, target);
        const safeEnd = Math.min(durationMs, Math.max(trimStartMs + 100, moveMs));
        setTrim(trimStartMs, safeEnd);
      };

      const onMouseUp = () => {
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    },
    [calculateTimestampFromEvent, setTrim, trimStartMs, durationMs]
  );

  // Ruler tick generation
  const ticks = useMemo(() => {
    const tickCount = Math.max(5, Math.min(30, Math.floor((durationMs / 1000) * zoom * 2)));
    const intervalMs = durationMs / tickCount;
    const items = [];

    for (let i = 0; i <= tickCount; i++) {
      const ms = Math.round(i * intervalMs);
      const isMajor = ms % 1000 < 50 || ms % 500 < 25;
      const percent = (ms / durationMs) * 100;
      items.push({ ms, percent, isMajor });
    }
    return items;
  }, [durationMs, zoom]);

  // Format millisecond timecode
  const formatTimecode = (ms: number) => {
    const totalSeconds = ms / 1000;
    const mins = Math.floor(totalSeconds / 60);
    const secs = Math.floor(totalSeconds % 60);
    const centis = Math.floor((ms % 1000) / 10);
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}.${centis.toString().padStart(2, '0')}`;
  };

  const playheadPercent = durationMs > 0 ? (currentTimeMs / durationMs) * 100 : 0;
  const trimStartPercent = durationMs > 0 ? (trimStartMs / durationMs) * 100 : 0;
  const trimEndPercent = durationMs > 0 ? (trimEndMs / durationMs) * 100 : 100;

  return (
    <div className={`flex flex-col bg-slate-900 border-t border-slate-800 text-slate-200 select-none ${className}`}>
      {/* Top Playback Toolbar */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-slate-800/80 bg-slate-950/60 backdrop-blur-sm text-xs">
        {/* Playback action controls */}
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            title="Rewind to trim start"
            onClick={() => {
              seek(trimStartMs);
              setActiveTimestampMs(trimStartMs);
            }}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
          >
            <RotateCcw className="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            title="Step 1 frame backward (ArrowLeft)"
            onClick={() => stepFrame(-1)}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
          >
            <SkipBack className="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            title={isPlaying ? 'Pause (Space)' : 'Play (Space)'}
            onClick={togglePlay}
            className="p-2 rounded-xl bg-blue-600 hover:bg-blue-500 active:bg-blue-700 text-white shadow-md shadow-blue-950/50 transition cursor-pointer"
          >
            {isPlaying ? (
              <Pause className="w-4 h-4 fill-current" />
            ) : (
              <Play className="w-4 h-4 fill-current ml-0.5" />
            )}
          </button>

          <button
            type="button"
            title="Step 1 frame forward (ArrowRight)"
            onClick={() => stepFrame(1)}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
          >
            <SkipForward className="w-3.5 h-3.5" />
          </button>

          <button
            type="button"
            title={`Loop playback: ${isLooping ? 'ON' : 'OFF'}`}
            onClick={() => setLooping(!isLooping)}
            className={`p-1.5 rounded-lg transition cursor-pointer ${
              isLooping ? 'bg-blue-600/20 text-blue-400' : 'text-slate-500 hover:text-slate-300 hover:bg-slate-800'
            }`}
          >
            <Repeat className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Timecode Readout */}
        <div className="flex items-center gap-3 font-mono font-semibold tracking-tight">
          <span className="text-white text-sm">{formatTimecode(currentTimeMs)}</span>
          <span className="text-slate-600">/</span>
          <span className="text-slate-400 text-xs">{formatTimecode(durationMs)}</span>

          <div className="hidden sm:flex items-center gap-1.5 pl-3 border-l border-slate-800 text-[11px] text-amber-400/90 font-medium">
            <Scissors className="w-3 h-3" />
            <span>
              Trim: {((trimEndMs - trimStartMs) / 1000).toFixed(2)}s
            </span>
          </div>
        </div>

        {/* Zoom Controls */}
        <div className="flex items-center gap-1">
          <button
            type="button"
            title="Zoom Out"
            onClick={() => setZoom(Math.max(0.5, zoom - 0.25))}
            className="p-1 rounded text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
          >
            <ZoomOut className="w-3.5 h-3.5" />
          </button>
          <span className="text-[11px] font-mono text-slate-400 w-9 text-center">
            {Math.round(zoom * 100)}%
          </span>
          <button
            type="button"
            title="Zoom In"
            onClick={() => setZoom(Math.min(3, zoom + 0.25))}
            className="p-1 rounded text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
          >
            <ZoomIn className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Main Timeline Scrubber Area */}
      <div className="relative px-6 py-2 select-none overflow-x-auto">
        <div
          ref={rulerRef}
          onMouseDown={handleRulerMouseDown}
          className="relative h-7 bg-slate-950/80 rounded-t-lg border border-slate-800 cursor-pointer overflow-hidden"
        >
          {/* Ruler tick marks */}
          {ticks.map(({ ms, percent, isMajor }) => (
            <div
              key={ms}
              className="absolute top-0 bottom-0 pointer-events-none flex flex-col justify-between"
              style={{ left: `${percent}%` }}
            >
              <div
                className={`w-[1px] ${
                  isMajor ? 'h-3 bg-slate-500' : 'h-1.5 bg-slate-700'
                }`}
              />
              {isMajor && (
                <span className="text-[9px] font-mono text-slate-400 -translate-x-1/2 select-none pb-0.5">
                  {(ms / 1000).toFixed(1)}s
                </span>
              )}
            </div>
          ))}

          {/* Shaded out-of-trim area (Left) */}
          <div
            className="absolute top-0 bottom-0 left-0 bg-black/60 pointer-events-none z-10"
            style={{ width: `${trimStartPercent}%` }}
          />

          {/* Shaded out-of-trim area (Right) */}
          <div
            className="absolute top-0 bottom-0 right-0 bg-black/60 pointer-events-none z-10"
            style={{ width: `${100 - trimEndPercent}%` }}
          />

          {/* Left Trim Handle */}
          <div
            role="separator"
            aria-label="Trim start"
            onMouseDown={handleLeftTrimMouseDown}
            className="trim-handle absolute top-0 bottom-0 w-3 -translate-x-1/2 bg-amber-500/90 hover:bg-amber-400 active:bg-amber-500 cursor-ew-resize z-30 flex items-center justify-center rounded-l shadow-lg"
            style={{ left: `${trimStartPercent}%` }}
            title={`Trim start: ${(trimStartMs / 1000).toFixed(2)}s`}
          >
            <div className="w-0.5 h-3.5 bg-slate-900 rounded-full" />
          </div>

          {/* Right Trim Handle */}
          <div
            role="separator"
            aria-label="Trim end"
            onMouseDown={handleRightTrimMouseDown}
            className="trim-handle absolute top-0 bottom-0 w-3 -translate-x-1/2 bg-amber-500/90 hover:bg-amber-400 active:bg-amber-500 cursor-ew-resize z-30 flex items-center justify-center rounded-r shadow-lg"
            style={{ left: `${trimEndPercent}%` }}
            title={`Trim end: ${(trimEndMs / 1000).toFixed(2)}s`}
          >
            <div className="w-0.5 h-3.5 bg-slate-900 rounded-full" />
          </div>

          {/* Playhead Needle Cursor Top */}
          <div
            className="absolute top-0 bottom-0 w-[2px] bg-red-500 z-40 pointer-events-none"
            style={{ left: `${playheadPercent}%` }}
          >
            <div className="absolute -top-1 -translate-x-1/2 w-3 h-3 bg-red-500 rotate-45 rounded-[1px] shadow-sm" />
          </div>
        </div>

        {/* Tracks Area */}
        <div
          ref={tracksContainerRef}
          className="relative bg-slate-950/40 rounded-b-lg border-x border-b border-slate-800 min-h-[72px] max-h-48 overflow-y-auto divide-y divide-slate-800/40"
        >
          {elements.length === 0 ? (
            <div className="h-16 flex items-center justify-center text-xs text-slate-500 italic">
              No annotations added yet. Use the toolbar above to annotate this recording.
            </div>
          ) : (
            elements.map((elem) => (
              <div key={elem.id} className="relative h-8 my-0.5">
                <TrackItem
                  element={elem}
                  durationMs={durationMs}
                  isSelected={elem.id === selectedElementId}
                  onSelect={(id) => selectElement(id)}
                  onUpdate={(id, patch) => updateElement(id, patch)}
                  onDelete={(id) => removeElement(id)}
                />
              </div>
            ))
          )}

          {/* Shaded out-of-trim overlay across tracks */}
          <div
            className="absolute top-0 bottom-0 left-0 bg-black/45 pointer-events-none z-10"
            style={{ width: `${trimStartPercent}%` }}
          />
          <div
            className="absolute top-0 bottom-0 right-0 bg-black/45 pointer-events-none z-10"
            style={{ width: `${100 - trimEndPercent}%` }}
          />

          {/* Playhead Needle Line across tracks */}
          <div
            className="absolute top-0 bottom-0 w-[2px] bg-red-500/80 z-20 pointer-events-none shadow-[0_0_8px_rgba(239,68,68,0.5)]"
            style={{ left: `${playheadPercent}%` }}
          />
        </div>
      </div>
    </div>
  );
};

export default TimelineScrubber;
