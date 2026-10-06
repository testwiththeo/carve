import React, { useState, useEffect, useRef, useCallback } from 'react';
import { X, RotateCcw, Maximize, Play } from 'lucide-react';

export interface SelectionRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface RegionSelectorProps {
  onConfirm?: (region: SelectionRegion) => void;
  onCancel?: () => void;
}

export const RegionSelector: React.FC<RegionSelectorProps> = ({
  onConfirm,
  onCancel,
}) => {
  const [isDragging, setIsDragging] = useState(false);
  const [startPoint, setStartPoint] = useState<{ x: number; y: number } | null>(null);
  const [region, setRegion] = useState<SelectionRegion | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close or hide window helper
  const handleCancel = useCallback(async () => {
    if (onCancel) {
      onCancel();
      return;
    }

    try {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      const win = getCurrentWebviewWindow();
      await win.hide();
    } catch {
      // Fallback in web preview
      setRegion(null);
    }
  }, [onCancel]);

  // Confirm selection and start recording
  const handleConfirm = useCallback(
    async (targetRegion: SelectionRegion) => {
      if (targetRegion.width <= 0 || targetRegion.height <= 0) return;
      setIsSubmitting(true);

      try {
        if (onConfirm) {
          onConfirm(targetRegion);
          return;
        }

        const { invoke } = await import('@tauri-apps/api/core');
        await invoke('start_recording', {
          region: {
            x: Math.round(targetRegion.x),
            y: Math.round(targetRegion.y),
            width: Math.round(targetRegion.width),
            height: Math.round(targetRegion.height),
          },
        });
      } catch (err) {
        console.error('Failed to start recording with region:', err);
      } finally {
        setIsSubmitting(false);
      }
    },
    [onConfirm]
  );

  // Keyboard shortcut listener: Enter = confirm, Escape = cancel
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        handleCancel();
      } else if (e.key === 'Enter' && region && region.width > 10 && region.height > 10) {
        e.preventDefault();
        handleConfirm(region);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [region, handleCancel, handleConfirm]);

  const handleMouseDown = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('.selector-toolbar')) {
      return;
    }
    const x = e.clientX;
    const y = e.clientY;
    setStartPoint({ x, y });
    setIsDragging(true);
    setRegion({ x, y, width: 0, height: 0 });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!isDragging || !startPoint) return;

    const currentX = e.clientX;
    const currentY = e.clientY;

    const x = Math.min(startPoint.x, currentX);
    const y = Math.min(startPoint.y, currentY);
    const width = Math.abs(currentX - startPoint.x);
    const height = Math.abs(currentY - startPoint.y);

    setRegion({ x, y, width, height });
  };

  const handleMouseUp = () => {
    if (!isDragging) return;
    setIsDragging(false);
    setStartPoint(null);
  };

  const applyPreset = (width: number, height: number) => {
    const screenW = window.innerWidth;
    const screenH = window.innerHeight;
    const targetW = Math.min(width, screenW);
    const targetH = Math.min(height, screenH);
    const x = Math.max(0, Math.round((screenW - targetW) / 2));
    const y = Math.max(0, Math.round((screenH - targetH) / 2));
    setRegion({ x, y, width: targetW, height: targetH });
  };

  const hasValidSelection = !!region && region.width >= 20 && region.height >= 20;

  return (
    <div
      ref={containerRef}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      className="fixed inset-0 select-none cursor-crosshair overflow-hidden bg-black/40"
      style={{ touchAction: 'none' }}
    >
      {/* Top Instruction Banner */}
      <div className="selector-toolbar absolute top-4 left-1/2 -translate-x-1/2 z-50 flex items-center gap-2 bg-slate-900/90 text-slate-100 px-4 py-2 rounded-full border border-slate-700/80 shadow-2xl backdrop-blur-md text-xs font-medium">
        <span>Drag to select capture region</span>
        <span className="text-slate-500">|</span>
        <button
          type="button"
          onClick={() => applyPreset(1280, 720)}
          className="hover:text-blue-400 px-1 py-0.5 rounded cursor-pointer transition-colors"
        >
          720p
        </button>
        <button
          type="button"
          onClick={() => applyPreset(1920, 1080)}
          className="hover:text-blue-400 px-1 py-0.5 rounded cursor-pointer transition-colors"
        >
          1080p
        </button>
        <button
          type="button"
          onClick={() => applyPreset(window.innerWidth, window.innerHeight)}
          className="hover:text-blue-400 px-1 py-0.5 rounded cursor-pointer transition-colors flex items-center gap-1"
        >
          <Maximize className="w-3 h-3" /> Full
        </button>
        <span className="text-slate-500">|</span>
        <button
          type="button"
          onClick={handleCancel}
          className="text-slate-400 hover:text-red-400 cursor-pointer flex items-center gap-1"
        >
          <X className="w-3.5 h-3.5" /> Esc
        </button>
      </div>

      {/* Selected Box cutout */}
      {region && region.width > 0 && region.height > 0 && (
        <div
          className="absolute border-2 border-red-500 rounded bg-transparent shadow-[0_0_0_99999px_rgba(0,0,0,0.45)] pointer-events-none"
          style={{
            left: `${region.x}px`,
            top: `${region.y}px`,
            width: `${region.width}px`,
            height: `${region.height}px`,
          }}
        >
          {/* Sizing pill */}
          <div className="absolute -top-7 left-0 bg-red-600 text-white text-[11px] font-mono font-semibold px-2 py-0.5 rounded shadow">
            {Math.round(region.width)} × {Math.round(region.height)}
            <span className="opacity-75 text-[10px] ml-1.5">
              ({Math.round(region.x)}, {Math.round(region.y)})
            </span>
          </div>

          {/* Action pill attached to bottom or top */}
          {hasValidSelection && !isDragging && (
            <div
              className="selector-toolbar absolute left-1/2 -translate-x-1/2 pointer-events-auto flex items-center gap-2 bg-slate-900/95 text-white px-3 py-1.5 rounded-lg border border-slate-700 shadow-2xl backdrop-blur-md text-xs"
              style={{
                top: region.y + region.height + 40 > window.innerHeight ? '-48px' : 'calc(100% + 8px)',
              }}
            >
              <button
                type="button"
                onClick={() => setRegion(null)}
                className="flex items-center gap-1 px-2 py-1 text-slate-300 hover:text-white rounded hover:bg-slate-800 cursor-pointer transition"
              >
                <RotateCcw className="w-3.5 h-3.5" /> Reset
              </button>
              <button
                type="button"
                disabled={isSubmitting}
                onClick={() => handleConfirm(region)}
                className="flex items-center gap-1.5 px-3 py-1 bg-red-600 hover:bg-red-500 active:bg-red-700 text-white font-medium rounded cursor-pointer transition shadow"
              >
                <Play className="w-3.5 h-3.5 fill-current" />
                {isSubmitting ? 'Starting...' : 'Record Selection'}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default RegionSelector;
