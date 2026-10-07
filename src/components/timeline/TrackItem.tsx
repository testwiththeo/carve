import React, { useRef, useCallback } from 'react';
import {
  ArrowRight,
  Square,
  Circle,
  Pencil,
  Type,
  Hash,
  Droplet,
  Highlighter,
  Trash2,
} from 'lucide-react';
import { VectorElement, VectorToolKind } from '../../types/vector';
import { clampTrackMove, clampTrackResize } from '../../stores/timelineStore';

export interface TrackItemProps {
  element: VectorElement;
  durationMs: number;
  isSelected?: boolean;
  onSelect?: (id: string) => void;
  onUpdate?: (id: string, patch: Partial<VectorElement>) => void;
  onDelete?: (id: string) => void;
}

const getToolIcon = (tool: VectorToolKind) => {
  switch (tool) {
    case 'arrow':
      return <ArrowRight className="w-3.5 h-3.5 text-blue-400" />;
    case 'rectangle':
      return <Square className="w-3.5 h-3.5 text-emerald-400" />;
    case 'ellipse':
      return <Circle className="w-3.5 h-3.5 text-indigo-400" />;
    case 'blur':
      return <Droplet className="w-3.5 h-3.5 text-amber-400" />;
    case 'text':
      return <Type className="w-3.5 h-3.5 text-pink-400" />;
    case 'counter_badge':
    case 'counterBadge':
      return <Hash className="w-3.5 h-3.5 text-purple-400" />;
    case 'pencil':
      return <Pencil className="w-3.5 h-3.5 text-orange-400" />;
    case 'highlight':
      return <Highlighter className="w-3.5 h-3.5 text-yellow-400" />;
    default:
      return <Square className="w-3.5 h-3.5 text-slate-400" />;
  }
};

const getToolLabel = (element: VectorElement) => {
  switch (element.tool) {
    case 'arrow':
      return 'Arrow';
    case 'rectangle':
      return 'Rectangle';
    case 'ellipse':
      return 'Circle';
    case 'blur':
      return 'Blur Redact';
    case 'text':
      return element.text ? `Text: "${element.text.slice(0, 12)}"` : 'Text';
    case 'counter_badge':
    case 'counterBadge':
      return `Badge #${element.count ?? 1}`;
    case 'pencil':
      return 'Drawing';
    case 'highlight':
      return 'Highlight';
    default:
      return element.tool;
  }
};

export const TrackItem: React.FC<TrackItemProps> = ({
  element,
  durationMs,
  isSelected = false,
  onSelect,
  onUpdate,
  onDelete,
}) => {
  const itemRef = useRef<HTMLDivElement>(null);

  // Position and width in percentages
  const safeDuration = Math.max(1, durationMs);
  const leftPercent = Math.max(0, Math.min(100, (element.start_ms / safeDuration) * 100));
  const widthPercent = Math.max(0.5, Math.min(100 - leftPercent, ((element.end_ms - element.start_ms) / safeDuration) * 100));

  // Drag moving the entire track block
  const handleBodyMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onSelect?.(element.id);

      const trackParent = itemRef.current?.parentElement;
      if (!trackParent) return;

      const parentRect = trackParent.getBoundingClientRect();
      const parentWidth = parentRect.width;
      if (parentWidth <= 0) return;

      const startClientX = e.clientX;
      const initialStartMs = element.start_ms;
      const initialEndMs = element.end_ms;
      const elementDuration = initialEndMs - initialStartMs;

      const onMouseMove = (moveEvent: MouseEvent) => {
        const deltaX = moveEvent.clientX - startClientX;
        const deltaMs = (deltaX / parentWidth) * safeDuration;
        const targetStartMs = Math.round(initialStartMs + deltaMs);

        const clamped = clampTrackMove(targetStartMs, elementDuration, safeDuration);
        onUpdate?.(element.id, clamped);
      };

      const onMouseUp = () => {
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    },
    [element.id, element.start_ms, element.end_ms, safeDuration, onSelect, onUpdate]
  );

  // Drag resizing the left handle (start_ms)
  const handleLeftHandleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onSelect?.(element.id);

      const trackParent = itemRef.current?.parentElement;
      if (!trackParent) return;

      const parentRect = trackParent.getBoundingClientRect();
      const parentWidth = parentRect.width;
      if (parentWidth <= 0) return;

      const startClientX = e.clientX;
      const initialStartMs = element.start_ms;
      const endMs = element.end_ms;

      const onMouseMove = (moveEvent: MouseEvent) => {
        const deltaX = moveEvent.clientX - startClientX;
        const deltaMs = (deltaX / parentWidth) * safeDuration;
        const targetStartMs = Math.round(initialStartMs + deltaMs);

        const clamped = clampTrackResize(targetStartMs, endMs, 50, safeDuration);
        onUpdate?.(element.id, { start_ms: clamped.start_ms });
      };

      const onMouseUp = () => {
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    },
    [element.id, element.start_ms, element.end_ms, safeDuration, onSelect, onUpdate]
  );

  // Drag resizing the right handle (end_ms)
  const handleRightHandleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      onSelect?.(element.id);

      const trackParent = itemRef.current?.parentElement;
      if (!trackParent) return;

      const parentRect = trackParent.getBoundingClientRect();
      const parentWidth = parentRect.width;
      if (parentWidth <= 0) return;

      const startClientX = e.clientX;
      const startMs = element.start_ms;
      const initialEndMs = element.end_ms;

      const onMouseMove = (moveEvent: MouseEvent) => {
        const deltaX = moveEvent.clientX - startClientX;
        const deltaMs = (deltaX / parentWidth) * safeDuration;
        const targetEndMs = Math.round(initialEndMs + deltaMs);

        const clamped = clampTrackResize(startMs, targetEndMs, 50, safeDuration);
        onUpdate?.(element.id, { end_ms: clamped.end_ms });
      };

      const onMouseUp = () => {
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };

      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
    },
    [element.id, element.start_ms, element.end_ms, safeDuration, onSelect, onUpdate]
  );

  const durationSec = ((element.end_ms - element.start_ms) / 1000).toFixed(2);
  const startSec = (element.start_ms / 1000).toFixed(2);
  const endSec = (element.end_ms / 1000).toFixed(2);

  return (
    <div
      ref={itemRef}
      role="button"
      tabIndex={0}
      onClick={(e) => {
        e.stopPropagation();
        onSelect?.(element.id);
      }}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          onSelect?.(element.id);
        } else if (e.key === 'Backspace' || e.key === 'Delete') {
          onDelete?.(element.id);
        }
      }}
      className={`absolute top-1 bottom-1 flex items-center justify-between rounded-md select-none group cursor-grab active:cursor-grabbing transition-shadow ${
        isSelected
          ? 'bg-blue-600/30 border border-blue-400 text-white shadow-md shadow-blue-900/40 ring-1 ring-blue-400'
          : 'bg-slate-800/90 hover:bg-slate-750 border border-slate-700/80 text-slate-200 hover:border-slate-600'
      }`}
      style={{
        left: `${leftPercent}%`,
        width: `${widthPercent}%`,
        minWidth: '24px',
      }}
      title={`${getToolLabel(element)}: ${startSec}s - ${endSec}s (${durationSec}s)`}
    >
      {/* Left resize handle */}
      <div
        role="separator"
        aria-label="Resize start"
        onMouseDown={handleLeftHandleMouseDown}
        className="w-2.5 h-full flex items-center justify-center cursor-ew-resize hover:bg-blue-500/40 rounded-l active:bg-blue-500 z-10 shrink-0"
        title="Drag to resize start time"
      >
        <span className="w-0.5 h-3 bg-slate-400/70 group-hover:bg-white rounded-full" />
      </div>

      {/* Main draggable track body */}
      <div
        onMouseDown={handleBodyMouseDown}
        className="flex-1 flex items-center gap-1.5 px-1.5 h-full overflow-hidden text-xs truncate"
      >
        <span className="shrink-0">{getToolIcon(element.tool)}</span>
        <span className="font-medium text-[11px] truncate tracking-tight">
          {getToolLabel(element)}
        </span>
        <span className="text-[10px] text-slate-400 font-mono hidden md:inline ml-auto pr-1">
          {durationSec}s
        </span>

        {/* Delete button (visible on hover / selected) */}
        {onDelete && (
          <button
            type="button"
            aria-label="Delete annotation"
            onClick={(e) => {
              e.stopPropagation();
              onDelete(element.id);
            }}
            className="opacity-0 group-hover:opacity-100 hover:bg-red-500/20 hover:text-red-300 p-0.5 rounded transition text-slate-400 cursor-pointer"
            title="Delete annotation"
          >
            <Trash2 className="w-3 h-3" />
          </button>
        )}
      </div>

      {/* Right resize handle */}
      <div
        role="separator"
        aria-label="Resize end"
        onMouseDown={handleRightHandleMouseDown}
        className="w-2.5 h-full flex items-center justify-center cursor-ew-resize hover:bg-blue-500/40 rounded-r active:bg-blue-500 z-10 shrink-0"
        title="Drag to resize end time"
      >
        <span className="w-0.5 h-3 bg-slate-400/70 group-hover:bg-white rounded-full" />
      </div>
    </div>
  );
};

export default TrackItem;
