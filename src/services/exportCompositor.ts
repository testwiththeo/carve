import {
  type VectorElement,
  queryActiveVectorElements,
} from '../types/vector';

export interface ExportProgress {
  currentFrame: number;
  totalFrames: number;
  percentage: number;
  stage: 'preparing' | 'compositing' | 'encoding' | 'complete' | 'error';
  errorMessage?: string;
}

export class ExportCancelledError extends Error {
  constructor(message: string = 'Export cancelled') {
    super(message);
    this.name = 'ExportCancelledError';
  }
}

export interface ExportCompositorOptions {
  trimStartMs: number;
  trimEndMs: number;
  fps: number;
  quality?: number;
  scale?: number;
  width?: number;
  height?: number;
  outputPath?: string;
  elements?: VectorElement[];
  baseFrames?: Array<{ timestampMs: number; image: CanvasImageSource | ImageData }>;
  getBaseFrame?: (timestampMs: number) => CanvasImageSource | ImageData | null;
  contextProvider?: (width: number, height: number) => CanvasRenderingContext2D | null;
  onProgress?: (progress: ExportProgress) => void;
  isCancelled?: () => boolean;
  abortSignal?: AbortSignal;
  useSessionCache?: boolean;
}

export interface CompositedFrame {
  frameIndex: number;
  timestampMs: number;
  width: number;
  height: number;
  rgbaData: Uint8ClampedArray | Uint8Array;
}

export interface ExportResult {
  success: boolean;
  outputPath: string;
  frameCount: number;
  durationMs: number;
  error?: string;
}

/**
 * Calculates ordered millisecond timestamps spaced at 1000/fps intervals within [trimStartMs, trimEndMs].
 * Handles inverted trim bounds and clamps safely.
 */
export function calculateFrameTimestamps(
  trimStartMs: number,
  trimEndMs: number,
  fps: number
): number[] {
  const minTrim = Math.min(trimStartMs, trimEndMs);
  const maxTrim = Math.max(trimStartMs, trimEndMs);
  const effectiveFps = Math.max(1, Math.min(120, fps));
  const stepMs = 1000 / effectiveFps;

  if (minTrim === maxTrim) {
    return [minTrim];
  }

  const timestamps: number[] = [];
  let current = minTrim;

  while (current <= maxTrim) {
    timestamps.push(Math.round(current));
    current += stepMs;
  }

  // Ensure last boundary frame is included if not already present
  if (timestamps.length === 0 || timestamps[timestamps.length - 1] < maxTrim) {
    timestamps.push(Math.round(maxTrim));
  }

  return timestamps;
}

/**
 * Parses CSS hex or rgba color into canvas-safe string.
 */
function resolveColor(color: string | undefined, fallback: string): string {
  if (!color || color === 'transparent') return fallback;
  return color;
}

/**
 * Rasterizes an individual vector annotation element onto an HTML5 2D canvas context.
 */
export function rasterizeVectorElement(
  ctx: CanvasRenderingContext2D,
  element: VectorElement,
  scale: number = 1
): void {
  ctx.save();

  const stroke = resolveColor(element.stroke, '#EF4444');
  const strokeWidth = (element.strokeWidth ?? 3) * scale;
  const fill = element.fill;

  ctx.strokeStyle = stroke;
  ctx.lineWidth = strokeWidth;

  const x = (element.x ?? 0) * scale;
  const y = (element.y ?? 0) * scale;
  const width = (element.width ?? 0) * scale;
  const height = (element.height ?? 0) * scale;

  switch (element.tool) {
    case 'rectangle': {
      if (fill && fill !== 'transparent') {
        ctx.fillStyle = fill;
        ctx.fillRect(x, y, width, height);
      }
      ctx.strokeRect(x, y, width, height);
      break;
    }

    case 'ellipse': {
      ctx.beginPath();
      const cx = x + width / 2;
      const cy = y + height / 2;
      const rx = Math.max(1, width / 2);
      const ry = Math.max(1, height / 2);
      if (typeof ctx.ellipse === 'function') {
        ctx.ellipse(cx, cy, rx, ry, 0, 0, 2 * Math.PI);
      } else {
        ctx.arc(cx, cy, rx, 0, 2 * Math.PI);
      }
      if (fill && fill !== 'transparent') {
        ctx.fillStyle = fill;
        ctx.fill();
      }
      ctx.stroke();
      break;
    }

    case 'arrow': {
      const pts = element.points ?? [0, 0, 100, 100];
      if (pts.length >= 4) {
        const x1 = pts[0] * scale;
        const y1 = pts[1] * scale;
        const x2 = pts[2] * scale;
        const y2 = pts[3] * scale;

        ctx.beginPath();
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();

        // Draw arrowhead
        const angle = Math.atan2(y2 - y1, x2 - x1);
        const headLen = Math.max(14 * scale, strokeWidth * 3.5);
        const arrowAngle = Math.PI / 6;

        ctx.beginPath();
        ctx.moveTo(x2, y2);
        ctx.lineTo(
          x2 - headLen * Math.cos(angle - arrowAngle),
          y2 - headLen * Math.sin(angle - arrowAngle)
        );
        ctx.moveTo(x2, y2);
        ctx.lineTo(
          x2 - headLen * Math.cos(angle + arrowAngle),
          y2 - headLen * Math.sin(angle + arrowAngle)
        );
        ctx.stroke();
      }
      break;
    }

    case 'pencil': {
      const pts = element.points ?? [];
      if (pts.length >= 4) {
        ctx.beginPath();
        ctx.moveTo(pts[0] * scale, pts[1] * scale);
        for (let i = 2; i < pts.length; i += 2) {
          ctx.lineTo(pts[i] * scale, pts[i + 1] * scale);
        }
        ctx.stroke();
      }
      break;
    }

    case 'counter_badge':
    case 'counterBadge': {
      const radius = 14 * scale;
      const badgeCount = String(element.count ?? 1);

      // Filled badge circle
      ctx.beginPath();
      ctx.fillStyle = stroke;
      ctx.arc(x, y, radius, 0, 2 * Math.PI);
      ctx.fill();

      // Number label inside badge
      ctx.fillStyle = '#FFFFFF';
      ctx.font = `bold ${Math.round(13 * scale)}px sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(badgeCount, x, y);
      break;
    }

    case 'text': {
      const text = element.text ?? 'Note';
      const fontSize = (element.fontSize ?? 18) * scale;
      const fontFamily = element.fontFamily ?? 'sans-serif';

      ctx.fillStyle = fill && fill !== 'transparent' ? fill : stroke;
      ctx.font = `${fontSize}px ${fontFamily}`;
      ctx.textAlign = 'start';
      ctx.textBaseline = 'top';
      ctx.fillText(text, x, y);
      break;
    }

    case 'blur': {
      // Pixelate effect on target subregion
      try {
        const blockSize = Math.max(4, Math.round(8 * scale));
        const imgData = ctx.getImageData(x, y, width, height);
        const data = imgData.data;
        const w = imgData.width;
        const h = imgData.height;

        for (let by = 0; by < h; by += blockSize) {
          for (let bx = 0; bx < w; bx += blockSize) {
            let r = 0, g = 0, b = 0, a = 0, count = 0;
            for (let dy = 0; dy < blockSize && by + dy < h; dy++) {
              for (let dx = 0; dx < blockSize && bx + dx < w; dx++) {
                const idx = ((by + dy) * w + (bx + dx)) * 4;
                r += data[idx];
                g += data[idx + 1];
                b += data[idx + 2];
                a += data[idx + 3];
                count++;
              }
            }
            if (count > 0) {
              const avgR = Math.round(r / count);
              const avgG = Math.round(g / count);
              const avgB = Math.round(b / count);
              const avgA = Math.round(a / count);
              for (let dy = 0; dy < blockSize && by + dy < h; dy++) {
                for (let dx = 0; dx < blockSize && bx + dx < w; dx++) {
                  const idx = ((by + dy) * w + (bx + dx)) * 4;
                  data[idx] = avgR;
                  data[idx + 1] = avgG;
                  data[idx + 2] = avgB;
                  data[idx + 3] = avgA;
                }
              }
            }
          }
        }
        ctx.putImageData(imgData, x, y);
      } catch {
        // Fallback: translucent overlay if getImageData is unpermitted
        ctx.fillStyle = 'rgba(128, 128, 128, 0.6)';
        ctx.fillRect(x, y, width, height);
      }
      break;
    }

    case 'highlight': {
      // Semi-transparent yellow highlighter
      ctx.fillStyle = fill || 'rgba(255, 235, 59, 0.4)';
      ctx.fillRect(x, y, width, height);
      break;
    }

    default:
      break;
  }

  ctx.restore();
}

/**
 * Creates or borrows an offscreen canvas context for rendering.
 */
function createOffscreenContext(
  width: number,
  height: number,
  provider?: (w: number, h: number) => CanvasRenderingContext2D | null
): CanvasRenderingContext2D | null {
  if (provider) {
    const custom = provider(width, height);
    if (custom) return custom;
  }

  if (typeof document !== 'undefined') {
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    return canvas.getContext('2d', { willReadFrequently: true });
  }

  return null;
}

/**
 * Finds the base screencast frame closest to targetMs.
 * Returns null if baseFrames is empty or undefined.
 */
export function findMatchingBaseFrame(
  baseFrames: Array<{ timestampMs: number; image: CanvasImageSource | ImageData }> | undefined,
  targetMs: number
): CanvasImageSource | ImageData | null {
  if (!baseFrames || baseFrames.length === 0) return null;
  if (baseFrames.length === 1) return baseFrames[0].image;

  let best = baseFrames[0];
  let minDiff = Math.abs(baseFrames[0].timestampMs - targetMs);

  for (let i = 1; i < baseFrames.length; i++) {
    const bf = baseFrames[i];
    const diff = Math.abs(bf.timestampMs - targetMs);
    if (diff < minDiff) {
      minDiff = diff;
      best = bf;
    }
  }

  return best.image;
}

/**
 * Type guard for ImageData or mock ImageData objects.
 */
function isImageData(img: unknown): img is ImageData {
  return (
    typeof img === 'object' &&
    img !== null &&
    'data' in img &&
    'width' in img &&
    'height' in img
  );
}

/**
 * Composites base frame with active vector elements onto an offscreen canvas at timestampMs.
 */
export function compositeFrame(
  ctx: CanvasRenderingContext2D,
  baseImage: CanvasImageSource | ImageData | null,
  elements: VectorElement[],
  timestampMs: number,
  options?: { width: number; height: number; scale?: number }
): ImageData {
  const width = options?.width ?? ctx.canvas?.width ?? 800;
  const height = options?.height ?? ctx.canvas?.height ?? 600;
  const scale = options?.scale ?? 1;

  if (baseImage) {
    if (isImageData(baseImage)) {
      ctx.putImageData(baseImage, 0, 0);
    } else {
      ctx.drawImage(baseImage, 0, 0, width, height);
    }
  } else {
    // Fill background with dark canvas neutral if no base frame supplied
    ctx.fillStyle = '#0F172A';
    ctx.fillRect(0, 0, width, height);
  }

  const activeElements = queryActiveVectorElements(elements, timestampMs);
  for (const elem of activeElements) {
    rasterizeVectorElement(ctx, elem, scale);
  }

  return ctx.getImageData(0, 0, width, height);
}

/**
 * Iterates through all timestamp slices between trimStartMs and trimEndMs,
 * evaluating active vector elements and rasterizing them over the base canvas.
 */
export async function compositeTimelineFrames(
  options: ExportCompositorOptions
): Promise<CompositedFrame[]> {
  const {
    trimStartMs,
    trimEndMs,
    fps,
    scale = 1,
    width = 960,
    height = 540,
    elements = [],
    baseFrames,
    getBaseFrame,
    contextProvider,
    onProgress,
    isCancelled,
    abortSignal,
  } = options;

  const checkCancelled = () => {
    if (isCancelled?.() || abortSignal?.aborted) {
      throw new ExportCancelledError('Export cancelled by user');
    }
  };

  checkCancelled();

  const targetW = Math.round(width * scale);
  const targetH = Math.round(height * scale);

  const ctx = createOffscreenContext(targetW, targetH, contextProvider);
  const timestamps = calculateFrameTimestamps(trimStartMs, trimEndMs, fps);
  const totalFrames = timestamps.length;
  const frames: CompositedFrame[] = [];

  onProgress?.({
    currentFrame: 0,
    totalFrames,
    percentage: 0,
    stage: 'preparing',
  });

  for (let idx = 0; idx < totalFrames; idx++) {
    checkCancelled();

    const timestampMs = timestamps[idx];

    // Render base frame if supplied via baseFrames or getBaseFrame
    const baseImage = getBaseFrame
      ? getBaseFrame(timestampMs)
      : findMatchingBaseFrame(baseFrames, timestampMs);

    let rgbaData: Uint8ClampedArray | Uint8Array;
    if (ctx) {
      const imgData = compositeFrame(ctx, baseImage, elements, timestampMs, {
        width: targetW,
        height: targetH,
        scale,
      });
      rgbaData = imgData.data;
    } else {
      // Mock fallback buffer for headless/Node environments
      rgbaData = new Uint8ClampedArray(targetW * targetH * 4);
    }

    frames.push({
      frameIndex: idx,
      timestampMs,
      width: targetW,
      height: targetH,
      rgbaData,
    });

    const percentage = Math.round(((idx + 1) / totalFrames) * 100);
    onProgress?.({
      currentFrame: idx + 1,
      totalFrames,
      percentage,
      stage: idx + 1 === totalFrames ? 'complete' : 'compositing',
    });
  }

  return frames;
}

/**
 * End-to-end export orchestrator.
 * Renders offscreen composited frames and invokes Tauri's native gifski export pipeline.
 */
export async function exportRecordingToGif(
  options: ExportCompositorOptions
): Promise<ExportResult> {
  const {
    trimStartMs,
    trimEndMs,
    fps,
    quality = 80,
    outputPath = 'screencast.gif',
    useSessionCache = false,
    elements = [],
    onProgress,
    isCancelled,
    abortSignal,
  } = options;

  const checkCancelled = () => {
    if (isCancelled?.() || abortSignal?.aborted) {
      throw new ExportCancelledError('Export cancelled by user');
    }
  };

  try {
    checkCancelled();

    onProgress?.({
      currentFrame: 0,
      totalFrames: 1,
      percentage: 5,
      stage: 'preparing',
    });

    let frameCount = 0;

    if (useSessionCache) {
      // Session cache export path: invoke native export_session_to_gif via export_gif command
      onProgress?.({
        currentFrame: 0,
        totalFrames: 1,
        percentage: 30,
        stage: 'encoding',
      });

      if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        const { invoke } = await import('@tauri-apps/api/core');

        if (elements.length > 0) {
          try {
            await invoke('update_vector_ledger', {
              ledger: { elements },
            });
          } catch {
            // Non-fatal ledger sync attempt
          }
        }

        checkCancelled();

        await invoke('export_gif', {
          outputPath,
          options: {
            fps,
            quality,
            loopCount: 0,
            width: options.width,
            height: options.height,
          },
          trimStartMs,
          trimEndMs,
        });
      }

      checkCancelled();
      frameCount = calculateFrameTimestamps(trimStartMs, trimEndMs, fps).length;
    } else {
      // 1. Offscreen composition of frames with base frames and vector overlays
      const compositedFrames = await compositeTimelineFrames({
        ...options,
        onProgress: (p) => {
          onProgress?.({
            ...p,
            percentage: Math.round(p.percentage * 0.7), // 0-70% for composition
            stage: 'compositing',
          });
        },
      });

      checkCancelled();
      frameCount = compositedFrames.length;

      onProgress?.({
        currentFrame: compositedFrames.length,
        totalFrames: compositedFrames.length,
        percentage: 75,
        stage: 'encoding',
      });

      // 2. Feed composited frames directly to Rust encode_raw_frames command
      if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        const { invoke } = await import('@tauri-apps/api/core');

        const rawFrames = compositedFrames.map((f) => ({
          timestampMs: f.timestampMs,
          width: f.width,
          height: f.height,
          rgbaData: Array.isArray(f.rgbaData) ? f.rgbaData : Array.from(f.rgbaData),
        }));

        checkCancelled();

        await invoke('encode_raw_frames', {
          outputPath,
          options: {
            fps,
            quality,
            loopCount: 0,
            width: options.width,
            height: options.height,
          },
          frames: rawFrames,
        });
      }
    }

    checkCancelled();

    onProgress?.({
      currentFrame: frameCount,
      totalFrames: frameCount,
      percentage: 100,
      stage: 'complete',
    });

    return {
      success: true,
      outputPath,
      frameCount,
      durationMs: Math.abs(trimEndMs - trimStartMs),
    };
  } catch (err: unknown) {
    const message = err instanceof Error ? err.message : String(err);
    onProgress?.({
      currentFrame: 0,
      totalFrames: 0,
      percentage: 0,
      stage: 'error',
      errorMessage: message,
    });
    throw err;
  }
}

/**
 * Convenience helper exporting from backend session cache directly.
 * Reads frames directly from local DiskFrameCache in Rust without JSON array IPC overhead.
 */
export async function exportSessionToGif(
  options: ExportCompositorOptions
): Promise<ExportResult>;
export async function exportSessionToGif(
  exportPath: string,
  exportOptions?: Partial<ExportCompositorOptions>,
  trimStartMs?: number,
  trimEndMs?: number
): Promise<ExportResult>;
export async function exportSessionToGif(
  optionsOrPath: ExportCompositorOptions | string,
  exportOptions?: Partial<ExportCompositorOptions>,
  trimStartMs?: number,
  trimEndMs?: number
): Promise<ExportResult> {
  if (typeof optionsOrPath === 'string') {
    return exportRecordingToGif({
      outputPath: optionsOrPath,
      fps: exportOptions?.fps ?? 30,
      quality: exportOptions?.quality ?? 85,
      scale: exportOptions?.scale ?? 1,
      width: exportOptions?.width,
      height: exportOptions?.height,
      elements: exportOptions?.elements ?? [],
      onProgress: exportOptions?.onProgress,
      abortSignal: exportOptions?.abortSignal,
      isCancelled: exportOptions?.isCancelled,
      trimStartMs: trimStartMs ?? exportOptions?.trimStartMs ?? 0,
      trimEndMs: trimEndMs ?? exportOptions?.trimEndMs ?? 5000,
      useSessionCache: true,
    });
  }

  return exportRecordingToGif({
    ...optionsOrPath,
    useSessionCache: true,
  });
}

/**
 * Class-based wrapper managing ongoing export sessions with cancellation and lifecycle events.
 */
export class ExportCompositor {
  private isCancelled = false;

  get cancelled(): boolean {
    return this.isCancelled;
  }

  cancel(): void {
    this.isCancelled = true;
  }

  async export(options: ExportCompositorOptions): Promise<ExportResult> {
    this.isCancelled = false;
    return exportRecordingToGif({
      ...options,
      isCancelled: () => this.isCancelled,
    });
  }

  async compositeFrames(options: ExportCompositorOptions): Promise<CompositedFrame[]> {
    this.isCancelled = false;
    return compositeTimelineFrames({
      ...options,
      isCancelled: () => this.isCancelled,
    });
  }
}
