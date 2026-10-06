import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  Square,
  Play,
  Pause,
  MousePointer,
  ArrowRight,
  RectangleHorizontal,
  Circle,
  Pencil,
  Type,
  Hash,
  Sparkles,
  Droplet,
  Eye,
  EyeOff,
} from 'lucide-react';
import { VectorStage } from '../components/canvas/VectorStage';
import { useAnnotationStore } from '../stores/annotationStore';
import { VectorElement, VectorToolKind } from '../types/vector';
import {
  AUTO_FADE_TIMEOUT_MS,
  EphemeralAnnotationManager,
  computeOverlayClickthrough,
  isWaylandEnvironment,
} from '../utils/overlayManager';

interface LiveOverlayProps {
  onStop?: () => void;
}

type ExtendedTool = VectorToolKind | 'pointer';

export const LiveOverlay: React.FC<LiveOverlayProps> = ({ onStop }) => {
  const {
    setActiveTool,
    strokeColor,
    setStrokeColor,
    removeElement,
  } = useAnnotationStore();

  const [currentTool, setCurrentTool] = useState<ExtendedTool>('pointer');
  const [manualClickthrough, setManualClickthrough] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [elapsedSec, setElapsedSec] = useState(0);

  // Presenter mode (ephemeral auto-fade after 1.5s) - default on Wayland
  const [isPresenterMode, setIsPresenterMode] = useState<boolean>(() => isWaylandEnvironment());

  // Window dimensions tracking
  const [dimensions, setDimensions] = useState({
    width: typeof window !== 'undefined' ? window.innerWidth : 800,
    height: typeof window !== 'undefined' ? window.innerHeight : 600,
  });

  const ephemeralManagerRef = useRef<EphemeralAnnotationManager>(new EphemeralAnnotationManager());

  useEffect(() => {
    const onResize = () => {
      setDimensions({ width: window.innerWidth, height: window.innerHeight });
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  // Clean up timers on unmount
  useEffect(() => {
    const manager = ephemeralManagerRef.current;
    return () => {
      manager.clear();
    };
  }, []);

  // Timer counter for recording duration
  useEffect(() => {
    if (isPaused) return;
    const interval = setInterval(() => {
      setElapsedSec((prev) => prev + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [isPaused]);

  // Sync clickthrough with backend
  const isClickthroughActive = computeOverlayClickthrough({
    activeTool: currentTool === 'pointer' ? null : currentTool,
    manualClickthrough,
  });

  useEffect(() => {
    import('@tauri-apps/api/core')
      .then(({ invoke }) => {
        invoke('set_overlay_clickthrough', { enabled: isClickthroughActive }).catch(() => {});
      })
      .catch(() => {});
  }, [isClickthroughActive]);

  // Tool selection handler
  const handleSelectTool = (tool: ExtendedTool) => {
    setCurrentTool(tool);
    if (tool !== 'pointer') {
      setActiveTool(tool);
    }
  };

  // Ephemeral stroke handler when presenter mode is active
  const handleDrawEnd = useCallback(
    (element: VectorElement) => {
      if (isPresenterMode) {
        ephemeralManagerRef.current.track(
          element.id,
          (id) => {
            removeElement(id);
          },
          true,
          AUTO_FADE_TIMEOUT_MS
        );
      }
    },
    [isPresenterMode, removeElement]
  );

  // Tauri command actions
  const handleStopRecording = async () => {
    try {
      if (onStop) {
        onStop();
        return;
      }
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('stop_recording');
    } catch (err) {
      console.error('Failed to stop recording:', err);
    }
  };

  const handleTogglePause = async () => {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      if (isPaused) {
        await invoke('resume_recording');
        setIsPaused(false);
      } else {
        await invoke('pause_recording');
        setIsPaused(true);
      }
    } catch (err) {
      console.error('Failed to toggle pause:', err);
    }
  };

  const handleToggleClickthrough = async () => {
    const nextVal = !manualClickthrough;
    setManualClickthrough(nextVal);
  };

  // Format elapsed time MM:SS
  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60)
      .toString()
      .padStart(2, '0');
    const s = (secs % 60).toString().padStart(2, '0');
    return `${m}:${s}`;
  };

  const paletteColors = ['#EF4444', '#3B82F6', '#10B981', '#F59E0B', '#FFFFFF'];

  return (
    <div className="fixed inset-0 overflow-hidden bg-transparent select-none">
      {/* Konva Vector Drawing Layer */}
      <div
        className="absolute inset-0"
        style={{
          pointerEvents: isClickthroughActive ? 'none' : 'auto',
        }}
      >
        <VectorStage
          width={dimensions.width}
          height={dimensions.height}
          readOnly={currentTool === 'pointer'}
          onDrawEnd={handleDrawEnd}
        />
      </div>

      {/* Floating Control Pill */}
      <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-50 pointer-events-auto">
        <div className="flex items-center gap-2 bg-slate-900/95 text-slate-100 px-3.5 py-2 rounded-2xl border border-slate-700/80 shadow-2xl backdrop-blur-md">
          {/* Status & Timer */}
          <div className="flex items-center gap-2 pr-2 border-r border-slate-700/80 text-xs font-mono">
            <span
              className={`w-2.5 h-2.5 rounded-full ${
                isPaused ? 'bg-amber-500' : 'bg-red-500 animate-pulse'
              }`}
            />
            <span className="font-semibold text-slate-200">{formatTime(elapsedSec)}</span>
          </div>

          {/* Pause / Resume button */}
          <button
            type="button"
            title={isPaused ? 'Resume recording' : 'Pause recording'}
            onClick={handleTogglePause}
            className={`p-1.5 rounded-lg cursor-pointer transition ${
              isPaused
                ? 'bg-amber-600/30 text-amber-300 hover:bg-amber-600/50'
                : 'hover:bg-slate-800 text-slate-300 hover:text-white'
            }`}
          >
            {isPaused ? <Play className="w-4 h-4 fill-current" /> : <Pause className="w-4 h-4 fill-current" />}
          </button>

          {/* Stop recording button */}
          <button
            type="button"
            title="Stop recording"
            onClick={handleStopRecording}
            className="p-1.5 rounded-lg bg-red-600/80 hover:bg-red-600 text-white cursor-pointer transition shadow"
          >
            <Square className="w-4 h-4 fill-current" />
          </button>

          <span className="text-slate-700">|</span>

          {/* Tool selectors */}
          <div className="flex items-center gap-0.5">
            <button
              type="button"
              title="Pointer (Pass-through mode)"
              onClick={() => handleSelectTool('pointer')}
              className={`p-1.5 rounded-lg cursor-pointer transition ${
                currentTool === 'pointer'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <MousePointer className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Arrow annotation"
              onClick={() => handleSelectTool('arrow')}
              className={`p-1.5 rounded-lg cursor-pointer transition ${
                currentTool === 'arrow'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <ArrowRight className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Rectangle annotation"
              onClick={() => handleSelectTool('rectangle')}
              className={`p-1.5 rounded-lg cursor-pointer transition ${
                currentTool === 'rectangle'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <RectangleHorizontal className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Ellipse annotation"
              onClick={() => handleSelectTool('ellipse')}
              className={`p-1.5 rounded-lg cursor-pointer transition ${
                currentTool === 'ellipse'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Circle className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Pencil / Freehand"
              onClick={() => handleSelectTool('pencil')}
              className={`p-1.5 rounded-lg cursor-pointer transition ${
                currentTool === 'pencil'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Pencil className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Text annotation"
              onClick={() => handleSelectTool('text')}
              className={`p-1.5 rounded-lg cursor-pointer transition ${
                currentTool === 'text'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Type className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Step Counter Badge"
              onClick={() => handleSelectTool('counter_badge')}
              className={`p-1.5 rounded-lg cursor-pointer transition ${
                currentTool === 'counter_badge' || currentTool === 'counterBadge'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Hash className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Blur / Redaction"
              onClick={() => handleSelectTool('blur')}
              className={`p-1.5 rounded-lg cursor-pointer transition ${
                currentTool === 'blur'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Droplet className="w-4 h-4" />
            </button>
          </div>

          <span className="text-slate-700">|</span>

          {/* Color palette */}
          <div className="flex items-center gap-1">
            {paletteColors.map((color) => (
              <button
                key={color}
                type="button"
                onClick={() => setStrokeColor(color)}
                className={`w-3.5 h-3.5 rounded-full cursor-pointer transition-transform ${
                  strokeColor === color ? 'scale-125 ring-2 ring-white ring-offset-1 ring-offset-slate-900' : 'hover:scale-110'
                }`}
                style={{ backgroundColor: color }}
              />
            ))}
          </div>

          <span className="text-slate-700">|</span>

          {/* Presenter Mode (1.5s Ephemeral Fade) */}
          <button
            type="button"
            title={`Presenter mode (1.5s auto-fade): ${isPresenterMode ? 'ON' : 'OFF'}`}
            onClick={() => setIsPresenterMode(!isPresenterMode)}
            className={`flex items-center gap-1 px-2 py-1 text-xs rounded-lg cursor-pointer transition ${
              isPresenterMode
                ? 'bg-purple-600/30 text-purple-300 border border-purple-500/50'
                : 'text-slate-400 hover:text-slate-300 hover:bg-slate-800'
            }`}
          >
            <Sparkles className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">1.5s</span>
          </button>

          {/* Manual Clickthrough toggle */}
          <button
            type="button"
            title={`Clickthrough: ${manualClickthrough ? 'Forced ON' : 'Auto'}`}
            onClick={handleToggleClickthrough}
            className={`p-1.5 rounded-lg cursor-pointer transition ${
              manualClickthrough
                ? 'bg-emerald-600/30 text-emerald-300 border border-emerald-500/40'
                : 'text-slate-400 hover:text-slate-300 hover:bg-slate-800'
            }`}
          >
            {manualClickthrough ? <Eye className="w-4 h-4" /> : <EyeOff className="w-4 h-4" />}
          </button>
        </div>
      </div>
    </div>
  );
};

export default LiveOverlay;
