import React, { useState, useRef, useEffect } from 'react';
import {
  Video,
  Crop,
  Download,
  RotateCcw,
  ArrowRight,
  Square,
  Circle,
  Pencil,
  Type,
  Hash,
  Droplet,
  Highlighter,
  Trash2,
  Undo2,
  Check,
  X,
  Film,
} from 'lucide-react';
import { VectorStage } from '../components/canvas/VectorStage';
import { TimelineScrubber } from '../components/timeline/TimelineScrubber';
import { useTimelineStore } from '../stores/timelineStore';
import { useAnnotationStore } from '../stores/annotationStore';
import { VectorToolKind } from '../types/vector';

export interface StudioViewProps {
  onRecordAgain?: () => void;
  onOpenSelector?: () => void;
}

export const StudioView: React.FC<StudioViewProps> = ({
  onRecordAgain,
  onOpenSelector,
}) => {
  const {
    durationMs,
    currentTimeMs,
    trimStartMs,
    trimEndMs,
    fps,
  } = useTimelineStore();

  const {
    activeTool,
    setActiveTool,
    strokeColor,
    setStrokeColor,
    strokeWidth,
    setStrokeWidth,
    clearElements,
    addElement,
    elements,
    removeElement,
    selectedElementId,
  } = useAnnotationStore();

  // Dialog and panel states
  const [showExportModal, setShowExportModal] = useState(false);
  const [showCropModal, setShowCropModal] = useState(false);

  // Export options state
  const [exportFps, setExportFps] = useState(fps);
  const [exportQuality, setExportQuality] = useState(85);
  const [exportScale, setExportScale] = useState(1);
  const [exportFormat, setExportFormat] = useState<'gif' | 'webp' | 'mp4'>('gif');
  const [isExporting, setIsExporting] = useState(false);
  const [exportCompleted, setExportCompleted] = useState(false);

  // Crop options state
  const [cropWidth, setCropWidth] = useState(1280);
  const [cropHeight, setCropHeight] = useState(720);

  // Preview dimensions
  const previewContainerRef = useRef<HTMLDivElement>(null);
  const [previewDims, setPreviewDims] = useState({ width: 960, height: 540 });

  useEffect(() => {
    const updateSize = () => {
      if (previewContainerRef.current) {
        const rect = previewContainerRef.current.getBoundingClientRect();
        // Maintain 16:9 aspect ratio or fit container
        const targetW = Math.max(320, Math.min(rect.width - 32, 1280));
        const targetH = Math.round((targetW * 9) / 16);
        setPreviewDims({ width: targetW, height: targetH });
      }
    };

    updateSize();
    window.addEventListener('resize', updateSize);
    return () => window.removeEventListener('resize', updateSize);
  }, []);

  // Retroactive quick add at current playhead
  const handleQuickAdd = (tool: VectorToolKind) => {
    setActiveTool(tool);
    const startMs = currentTimeMs;
    const endMs = Math.min(durationMs, startMs + 2500);

    const centerX = Math.round(previewDims.width / 2);
    const centerY = Math.round(previewDims.height / 2);

    switch (tool) {
      case 'arrow':
        addElement({
          tool: 'arrow',
          points: [centerX - 60, centerY + 40, centerX + 60, centerY - 40],
          start_ms: startMs,
          end_ms: endMs,
        });
        break;
      case 'rectangle':
        addElement({
          tool: 'rectangle',
          x: centerX - 75,
          y: centerY - 50,
          width: 150,
          height: 100,
          start_ms: startMs,
          end_ms: endMs,
        });
        break;
      case 'blur':
        addElement({
          tool: 'blur',
          x: centerX - 80,
          y: centerY - 40,
          width: 160,
          height: 80,
          start_ms: startMs,
          end_ms: endMs,
        });
        break;
      case 'text':
        addElement({
          tool: 'text',
          x: centerX - 50,
          y: centerY - 15,
          text: 'Note',
          fontSize: 22,
          start_ms: startMs,
          end_ms: endMs,
        });
        break;
      case 'counter_badge':
      case 'counterBadge':
        addElement({
          tool: 'counter_badge',
          x: centerX,
          y: centerY,
          start_ms: startMs,
          end_ms: endMs,
        });
        break;
      default:
        break;
    }
  };

  const handleRecordAgain = () => {
    if (onRecordAgain) {
      onRecordAgain();
    } else if (onOpenSelector) {
      onOpenSelector();
    } else {
      // Tauri fallback trigger
      import('@tauri-apps/api/webviewWindow')
        .then(async ({ WebviewWindow }) => {
          const selectorWin = await WebviewWindow.getByLabel('selector');
          if (selectorWin) {
            await selectorWin.show();
            await selectorWin.setFocus();
          }
        })
        .catch(() => {});
    }
  };

  const handleStartExport = async () => {
    setIsExporting(true);
    setExportCompleted(false);

    // Call native export or simulate progress
    setTimeout(() => {
      setIsExporting(false);
      setExportCompleted(true);
    }, 1500);
  };

  const paletteColors = [
    '#EF4444', // Red
    '#3B82F6', // Blue
    '#10B981', // Green
    '#F59E0B', // Amber
    '#8B5CF6', // Purple
    '#FFFFFF', // White
  ];

  const strokeWidths = [2, 4, 6, 8];

  const trimDurationSec = ((trimEndMs - trimStartMs) / 1000).toFixed(2);
  const totalFrames = Math.round(((trimEndMs - trimStartMs) / 1000) * exportFps);

  return (
    <div className="flex flex-col h-screen w-screen bg-slate-950 text-slate-100 overflow-hidden select-none font-sans">
      {/* Top Action Bar */}
      <header className="flex items-center justify-between px-5 py-3 border-b border-slate-800/80 bg-slate-900/90 backdrop-blur-md z-30 shrink-0">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-red-600/20 text-red-400 rounded-xl border border-red-500/30">
            <Film className="w-5 h-5" />
          </div>
          <div>
            <h1 className="text-sm font-bold tracking-tight text-white flex items-center gap-2">
              GIF Studio
              <span className="text-[10px] font-mono font-medium px-1.5 py-0.5 rounded bg-slate-800 text-slate-400 border border-slate-700">
                {cropWidth} × {cropHeight}
              </span>
            </h1>
            <p className="text-[11px] text-slate-400">
              Trim clip, add interactive vector annotations & export
            </p>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={handleRecordAgain}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded-xl border border-slate-700 transition cursor-pointer"
          >
            <RotateCcw className="w-3.5 h-3.5 text-slate-400" />
            <span>Record Again</span>
          </button>

          <button
            type="button"
            onClick={() => setShowCropModal(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded-xl border border-slate-700 transition cursor-pointer"
          >
            <Crop className="w-3.5 h-3.5 text-blue-400" />
            <span>Crop</span>
          </button>

          <button
            type="button"
            onClick={() => setShowExportModal(true)}
            className="flex items-center gap-1.5 px-4 py-1.5 bg-gradient-to-r from-red-600 to-red-500 hover:from-red-500 hover:to-red-400 active:from-red-700 active:to-red-600 text-white text-xs font-semibold rounded-xl shadow-lg shadow-red-950/60 transition cursor-pointer"
          >
            <Download className="w-4 h-4" />
            <span>Export GIF</span>
          </button>
        </div>
      </header>

      {/* Main Studio Middle Area */}
      <div className="flex-1 flex flex-col items-center justify-center p-4 bg-slate-950/60 relative overflow-hidden min-h-0">
        {/* Flameshot-Style Annotation Floating Toolbar */}
        <div className="absolute top-6 left-1/2 -translate-x-1/2 z-40 flex items-center gap-1.5 bg-slate-900/95 border border-slate-700/80 px-3 py-1.5 rounded-2xl shadow-2xl backdrop-blur-xl text-xs">
          {/* Tool buttons */}
          <div className="flex items-center gap-0.5 pr-2 border-r border-slate-800">
            <button
              type="button"
              title="Arrow tool"
              onClick={() => setActiveTool('arrow')}
              className={`p-1.5 rounded-lg transition cursor-pointer ${
                activeTool === 'arrow'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <ArrowRight className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Rectangle tool"
              onClick={() => setActiveTool('rectangle')}
              className={`p-1.5 rounded-lg transition cursor-pointer ${
                activeTool === 'rectangle'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Square className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Circle / Ellipse tool"
              onClick={() => setActiveTool('ellipse')}
              className={`p-1.5 rounded-lg transition cursor-pointer ${
                activeTool === 'ellipse'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Circle className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Pencil / Freehand drawing"
              onClick={() => setActiveTool('pencil')}
              className={`p-1.5 rounded-lg transition cursor-pointer ${
                activeTool === 'pencil'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Pencil className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Text annotation"
              onClick={() => setActiveTool('text')}
              className={`p-1.5 rounded-lg transition cursor-pointer ${
                activeTool === 'text'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Type className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Step Counter Badge"
              onClick={() => setActiveTool('counter_badge')}
              className={`p-1.5 rounded-lg transition cursor-pointer ${
                activeTool === 'counter_badge' || activeTool === 'counterBadge'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Hash className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Blur / Redaction"
              onClick={() => setActiveTool('blur')}
              className={`p-1.5 rounded-lg transition cursor-pointer ${
                activeTool === 'blur'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Droplet className="w-4 h-4" />
            </button>

            <button
              type="button"
              title="Highlight tool"
              onClick={() => setActiveTool('highlight')}
              className={`p-1.5 rounded-lg transition cursor-pointer ${
                activeTool === 'highlight'
                  ? 'bg-blue-600 text-white shadow'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
              }`}
            >
              <Highlighter className="w-4 h-4" />
            </button>
          </div>

          {/* Color palette */}
          <div className="flex items-center gap-1 px-2 border-r border-slate-800">
            {paletteColors.map((color) => (
              <button
                key={color}
                type="button"
                onClick={() => setStrokeColor(color)}
                className={`w-3.5 h-3.5 rounded-full cursor-pointer transition-transform ${
                  strokeColor === color
                    ? 'scale-125 ring-2 ring-white ring-offset-1 ring-offset-slate-900'
                    : 'hover:scale-110'
                }`}
                style={{ backgroundColor: color }}
              />
            ))}
          </div>

          {/* Stroke width selector */}
          <div className="flex items-center gap-1 px-2 border-r border-slate-800">
            {strokeWidths.map((w) => (
              <button
                key={w}
                type="button"
                onClick={() => setStrokeWidth(w)}
                className={`px-1.5 py-0.5 text-[10px] rounded font-mono cursor-pointer transition ${
                  strokeWidth === w
                    ? 'bg-blue-600 text-white font-bold'
                    : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
                }`}
              >
                {w}px
              </button>
            ))}
          </div>

          {/* Quick Add buttons at playhead */}
          <div className="flex items-center gap-1 pl-1">
            <button
              type="button"
              title="Add Arrow at playhead"
              onClick={() => handleQuickAdd('arrow')}
              className="px-2 py-1 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded text-[11px] font-medium transition cursor-pointer flex items-center gap-1"
            >
              + Arrow
            </button>
            <button
              type="button"
              title="Add Blur at playhead"
              onClick={() => handleQuickAdd('blur')}
              className="px-2 py-1 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded text-[11px] font-medium transition cursor-pointer flex items-center gap-1"
            >
              + Blur
            </button>

            {selectedElementId && (
              <button
                type="button"
                title="Delete selected element"
                onClick={() => removeElement(selectedElementId)}
                className="p-1 rounded text-red-400 hover:bg-red-500/20 hover:text-red-300 transition cursor-pointer ml-1"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            )}

            {elements.length > 0 && (
              <button
                type="button"
                title="Clear all annotations"
                onClick={clearElements}
                className="p-1 rounded text-slate-500 hover:text-slate-300 hover:bg-slate-800 transition cursor-pointer"
              >
                <Undo2 className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </div>

        {/* Video Preview Canvas & Vector Annotation Overlay */}
        <div
          ref={previewContainerRef}
          className="relative rounded-xl border border-slate-800/90 bg-slate-900 shadow-2xl overflow-hidden flex items-center justify-center"
          style={{
            width: `${previewDims.width}px`,
            height: `${previewDims.height}px`,
          }}
        >
          {/* Simulated Recorded Video Frame Canvas */}
          <div className="absolute inset-0 bg-gradient-to-br from-slate-900 via-slate-950 to-slate-900 flex flex-col items-center justify-center pointer-events-none select-none">
            {/* Subtle grid pattern background */}
            <div className="absolute inset-0 opacity-15 bg-[radial-gradient(#38bdf8_1px,transparent_1px)] [background-size:16px_16px]" />

            <div className="flex flex-col items-center gap-2 text-slate-500 text-center z-0">
              <Video className="w-8 h-8 opacity-40 text-blue-400" />
              <span className="text-xs font-mono">
                Frame Preview @ {(currentTimeMs / 1000).toFixed(2)}s
              </span>
              <span className="text-[10px] text-slate-600">
                {cropWidth} × {cropHeight}px ({fps} fps)
              </span>
            </div>
          </div>

          {/* Konva VectorStage active for current timestamp */}
          <div className="absolute inset-0 z-20">
            <VectorStage
              width={previewDims.width}
              height={previewDims.height}
              activeTimestampMs={currentTimeMs}
            />
          </div>
        </div>
      </div>

      {/* Timeline Scrubber Component at the Bottom */}
      <div className="shrink-0 z-30">
        <TimelineScrubber />
      </div>

      {/* Export GIF Modal */}
      {showExportModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-md bg-slate-900 border border-slate-700/80 rounded-2xl p-6 shadow-2xl text-slate-100 space-y-5 animate-in fade-in zoom-in-95">
            <div className="flex items-center justify-between pb-3 border-b border-slate-800">
              <div className="flex items-center gap-2">
                <Download className="w-5 h-5 text-red-500" />
                <h3 className="font-semibold text-white">Export Recording</h3>
              </div>
              <button
                type="button"
                onClick={() => setShowExportModal(false)}
                className="p-1 rounded-lg hover:bg-slate-800 text-slate-400 hover:text-white cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Options Form */}
            <div className="space-y-4 text-xs">
              {/* Output format */}
              <div>
                <label className="block text-slate-400 mb-1.5 font-medium">Output Format</label>
                <div className="grid grid-cols-3 gap-2">
                  {(['gif', 'webp', 'mp4'] as const).map((fmt) => (
                    <button
                      key={fmt}
                      type="button"
                      onClick={() => setExportFormat(fmt)}
                      className={`py-2 rounded-xl font-medium uppercase tracking-wider text-center border cursor-pointer transition ${
                        exportFormat === fmt
                          ? 'bg-blue-600 text-white border-blue-500 shadow'
                          : 'bg-slate-800/80 text-slate-300 border-slate-700 hover:bg-slate-800'
                      }`}
                    >
                      {fmt}
                    </button>
                  ))}
                </div>
              </div>

              {/* FPS selector */}
              <div>
                <div className="flex justify-between text-slate-400 mb-1.5 font-medium">
                  <span>Frame Rate</span>
                  <span className="text-white font-mono">{exportFps} FPS</span>
                </div>
                <div className="flex gap-2">
                  {[15, 24, 30, 50].map((rate) => (
                    <button
                      key={rate}
                      type="button"
                      onClick={() => setExportFps(rate)}
                      className={`flex-1 py-1.5 rounded-lg border text-center font-mono cursor-pointer transition ${
                        exportFps === rate
                          ? 'bg-slate-700 text-white border-blue-400'
                          : 'bg-slate-800 text-slate-400 border-slate-700 hover:text-slate-200'
                      }`}
                    >
                      {rate}
                    </button>
                  ))}
                </div>
              </div>

              {/* Quality slider */}
              <div>
                <div className="flex justify-between text-slate-400 mb-1.5 font-medium">
                  <span>Quality (gifski)</span>
                  <span className="text-white font-mono">{exportQuality}%</span>
                </div>
                <input
                  type="range"
                  min="10"
                  max="100"
                  value={exportQuality}
                  onChange={(e) => setExportQuality(Number(e.target.value))}
                  className="w-full accent-red-500 cursor-pointer"
                />
              </div>

              {/* Scale / Resolution */}
              <div>
                <div className="flex justify-between text-slate-400 mb-1.5 font-medium">
                  <span>Resolution Scale</span>
                  <span className="text-white font-mono">{Math.round(exportScale * 100)}% ({Math.round(cropWidth * exportScale)} × {Math.round(cropHeight * exportScale)})</span>
                </div>
                <div className="grid grid-cols-3 gap-2">
                  {[
                    { label: '100% (Native)', val: 1 },
                    { label: '75%', val: 0.75 },
                    { label: '50% (Compact)', val: 0.5 },
                  ].map((s) => (
                    <button
                      key={s.val}
                      type="button"
                      onClick={() => setExportScale(s.val)}
                      className={`py-1.5 rounded-lg border text-center cursor-pointer transition ${
                        exportScale === s.val
                          ? 'bg-slate-700 text-white border-blue-400'
                          : 'bg-slate-800 text-slate-400 border-slate-700 hover:text-slate-200'
                      }`}
                    >
                      {s.label}
                    </button>
                  ))}
                </div>
              </div>

              {/* Summary Stats */}
              <div className="p-3 bg-slate-950/70 border border-slate-800 rounded-xl space-y-1 text-slate-400 text-[11px]">
                <div className="flex justify-between">
                  <span>Trim Duration:</span>
                  <span className="text-white font-mono font-medium">{trimDurationSec}s</span>
                </div>
                <div className="flex justify-between">
                  <span>Estimated Total Frames:</span>
                  <span className="text-white font-mono font-medium">{totalFrames} frames</span>
                </div>
                <div className="flex justify-between">
                  <span>Active Vector Annotations:</span>
                  <span className="text-white font-mono font-medium">{elements.length} elements</span>
                </div>
              </div>
            </div>

            {/* Actions */}
            <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
              <button
                type="button"
                onClick={() => setShowExportModal(false)}
                className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-xl text-xs font-medium cursor-pointer transition"
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={isExporting}
                onClick={handleStartExport}
                className="flex items-center gap-2 px-5 py-2 bg-red-600 hover:bg-red-500 active:bg-red-700 text-white rounded-xl text-xs font-semibold shadow-lg shadow-red-950/50 cursor-pointer transition disabled:opacity-50"
              >
                {isExporting ? (
                  <>
                    <span className="w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                    <span>Rendering GIF...</span>
                  </>
                ) : exportCompleted ? (
                  <>
                    <Check className="w-4 h-4 text-emerald-300" />
                    <span>Exported Successfully!</span>
                  </>
                ) : (
                  <>
                    <Download className="w-4 h-4" />
                    <span>Start Export</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Crop Modal */}
      {showCropModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4">
          <div className="w-full max-w-sm bg-slate-900 border border-slate-700/80 rounded-2xl p-6 shadow-2xl text-slate-100 space-y-4">
            <div className="flex items-center justify-between pb-3 border-b border-slate-800">
              <div className="flex items-center gap-2">
                <Crop className="w-5 h-5 text-blue-400" />
                <h3 className="font-semibold text-white">Crop Framing</h3>
              </div>
              <button
                type="button"
                onClick={() => setShowCropModal(false)}
                className="p-1 rounded-lg hover:bg-slate-800 text-slate-400 hover:text-white cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-3 text-xs">
              <div>
                <label className="block text-slate-400 mb-1">Width (px)</label>
                <input
                  type="number"
                  value={cropWidth}
                  onChange={(e) => setCropWidth(Math.max(100, Number(e.target.value)))}
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-white font-mono"
                />
              </div>

              <div>
                <label className="block text-slate-400 mb-1">Height (px)</label>
                <input
                  type="number"
                  value={cropHeight}
                  onChange={(e) => setCropHeight(Math.max(100, Number(e.target.value)))}
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-white font-mono"
                />
              </div>

              <div className="pt-2">
                <span className="block text-slate-400 mb-1.5">Presets</span>
                <div className="flex gap-2">
                  {[
                    { label: '720p', w: 1280, h: 720 },
                    { label: '1080p', w: 1920, h: 1080 },
                    { label: 'Square', w: 800, h: 800 },
                  ].map((p) => (
                    <button
                      key={p.label}
                      type="button"
                      onClick={() => {
                        setCropWidth(p.w);
                        setCropHeight(p.h);
                      }}
                      className="flex-1 py-1 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded border border-slate-700 text-center cursor-pointer transition"
                    >
                      {p.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex justify-end gap-2 pt-3 border-t border-slate-800">
              <button
                type="button"
                onClick={() => setShowCropModal(false)}
                className="px-4 py-1.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg text-xs font-semibold cursor-pointer transition"
              >
                Apply Crop
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default StudioView;
