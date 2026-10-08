import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  calculateFrameTimestamps,
  rasterizeVectorElement,
  compositeTimelineFrames,
  exportRecordingToGif,
  exportSessionToGif,
  findMatchingBaseFrame,
  ExportCompositor,
  ExportCancelledError,
  type ExportCompositorOptions,
  type ExportProgress,
} from '../src/services/exportCompositor';
import type { VectorElement } from '../src/types/vector';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Minimal CanvasRenderingContext2D mock for Node.js test environment
function createMockContext(): CanvasRenderingContext2D {
  return {
    save: vi.fn(),
    restore: vi.fn(),
    beginPath: vi.fn(),
    closePath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    stroke: vi.fn(),
    fill: vi.fn(),
    strokeRect: vi.fn(),
    fillRect: vi.fn(),
    clearRect: vi.fn(),
    arc: vi.fn(),
    ellipse: vi.fn(),
    fillText: vi.fn(),
    drawImage: vi.fn(),
    getImageData: vi.fn().mockReturnValue({
      data: new Uint8ClampedArray(100 * 100 * 4),
      width: 100,
      height: 100,
    }),
    putImageData: vi.fn(),
    canvas: {
      width: 100,
      height: 100,
    } as HTMLCanvasElement,
    fillStyle: '#000000',
    strokeStyle: '#000000',
    lineWidth: 1,
    font: '10px sans-serif',
    textAlign: 'start',
    textBaseline: 'alphabetic',
    filter: 'none',
  } as unknown as CanvasRenderingContext2D;
}

describe('exportCompositor: calculateFrameTimestamps', () => {
  it('generates timestamps at step 1000/fps from trimStart to trimEnd', () => {
    const timestamps = calculateFrameTimestamps(0, 1000, 10);
    // At 10 fps, step is 100ms. [0, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]
    expect(timestamps.length).toBe(11);
    expect(timestamps[0]).toBe(0);
    expect(timestamps[1]).toBe(100);
    expect(timestamps[timestamps.length - 1]).toBe(1000);
  });

  it('normalizes inverted trim ranges', () => {
    const timestamps = calculateFrameTimestamps(1000, 0, 10);
    expect(timestamps.length).toBe(11);
    expect(timestamps[0]).toBe(0);
    expect(timestamps[timestamps.length - 1]).toBe(1000);
  });

  it('handles single-point/zero duration range', () => {
    const timestamps = calculateFrameTimestamps(500, 500, 30);
    expect(timestamps).toEqual([500]);
  });

  it('handles arbitrary fps step rounding', () => {
    const timestamps = calculateFrameTimestamps(0, 300, 30);
    // 30 fps is ~33.33ms step
    expect(timestamps.length).toBeGreaterThan(5);
    expect(timestamps[0]).toBe(0);
  });
});

describe('exportCompositor: rasterizeVectorElement', () => {
  let ctx: CanvasRenderingContext2D;

  beforeEach(() => {
    ctx = createMockContext();
  });

  it('rasterizes rectangle element', () => {
    const elem: VectorElement = {
      id: 'rect_1',
      tool: 'rectangle',
      start_ms: 0,
      end_ms: 1000,
      x: 10,
      y: 10,
      width: 50,
      height: 40,
      stroke: '#EF4444',
      strokeWidth: 2,
      fill: 'rgba(255, 0, 0, 0.2)',
    };
    rasterizeVectorElement(ctx, elem);
    expect(ctx.strokeRect).toHaveBeenCalledWith(10, 10, 50, 40);
    expect(ctx.fillRect).toHaveBeenCalledWith(10, 10, 50, 40);
  });

  it('rasterizes ellipse element', () => {
    const elem: VectorElement = {
      id: 'ellipse_1',
      tool: 'ellipse',
      start_ms: 0,
      end_ms: 1000,
      x: 50,
      y: 50,
      width: 40,
      height: 30,
      stroke: '#3B82F6',
    };
    rasterizeVectorElement(ctx, elem);
    expect(ctx.beginPath).toHaveBeenCalled();
    expect(ctx.stroke).toHaveBeenCalled();
  });

  it('rasterizes arrow element', () => {
    const elem: VectorElement = {
      id: 'arrow_1',
      tool: 'arrow',
      start_ms: 0,
      end_ms: 1000,
      points: [10, 10, 90, 90],
      stroke: '#10B981',
      strokeWidth: 4,
    };
    rasterizeVectorElement(ctx, elem);
    expect(ctx.beginPath).toHaveBeenCalled();
    expect(ctx.moveTo).toHaveBeenCalledWith(10, 10);
    expect(ctx.lineTo).toHaveBeenCalledWith(90, 90);
    expect(ctx.stroke).toHaveBeenCalled();
  });

  it('rasterizes text element', () => {
    const elem: VectorElement = {
      id: 'text_1',
      tool: 'text',
      start_ms: 0,
      end_ms: 1000,
      x: 20,
      y: 30,
      text: 'Sample Note',
      fontSize: 16,
      fill: '#FFFFFF',
    };
    rasterizeVectorElement(ctx, elem);
    expect(ctx.fillText).toHaveBeenCalledWith('Sample Note', 20, 30);
  });

  it('rasterizes counter badge element', () => {
    const elem: VectorElement = {
      id: 'badge_1',
      tool: 'counter_badge',
      start_ms: 0,
      end_ms: 1000,
      x: 40,
      y: 40,
      count: 2,
    };
    rasterizeVectorElement(ctx, elem);
    expect(ctx.arc).toHaveBeenCalled();
    expect(ctx.fillText).toHaveBeenCalledWith('2', 40, 40);
  });

  it('rasterizes blur / highlight element without error', () => {
    const blurElem: VectorElement = {
      id: 'blur_1',
      tool: 'blur',
      start_ms: 0,
      end_ms: 1000,
      x: 10,
      y: 10,
      width: 30,
      height: 30,
    };
    const highlightElem: VectorElement = {
      id: 'high_1',
      tool: 'highlight',
      start_ms: 0,
      end_ms: 1000,
      x: 20,
      y: 20,
      width: 40,
      height: 20,
    };
    expect(() => rasterizeVectorElement(ctx, blurElem)).not.toThrow();
    expect(() => rasterizeVectorElement(ctx, highlightElem)).not.toThrow();
  });
});

describe('exportCompositor: compositeTimelineFrames and exportRecordingToGif', () => {
  it('iterates through timestamps and triggers progress callbacks', async () => {
    const progressUpdates: ExportProgress[] = [];
    const elements: VectorElement[] = [
      {
        id: '1',
        tool: 'rectangle',
        start_ms: 0,
        end_ms: 300,
        x: 10,
        y: 10,
        width: 20,
        height: 20,
      },
    ];

    const ctx = createMockContext();
    const frames = await compositeTimelineFrames({
      trimStartMs: 0,
      trimEndMs: 300,
      fps: 10,
      width: 100,
      height: 100,
      elements,
      contextProvider: () => ctx,
      onProgress: (p) => progressUpdates.push(p),
    });

    expect(frames.length).toBe(4); // 0, 100, 200, 300
    expect(progressUpdates.length).toBeGreaterThanOrEqual(4);
    expect(progressUpdates[progressUpdates.length - 1].percentage).toBe(100);
    expect(progressUpdates[progressUpdates.length - 1].stage).toBe('complete');
  });

  it('exportRecordingToGif runs end-to-end pipeline cleanly', async () => {
    const onProgress = vi.fn();
    const result = await exportRecordingToGif({
      outputPath: '/tmp/test_export.gif',
      trimStartMs: 0,
      trimEndMs: 200,
      fps: 10,
      quality: 80,
      elements: [],
      contextProvider: () => createMockContext(),
      onProgress,
    });

    expect(result.success).toBe(true);
    expect(result.frameCount).toBe(3); // 0, 100, 200
    expect(onProgress).toHaveBeenCalled();
  });
});

describe('exportCompositor: Base Frame Rendering', () => {
  it('findMatchingBaseFrame selects closest frame by timestamp', () => {
    const mockImg1 = {} as CanvasImageSource;
    const mockImg2 = {} as CanvasImageSource;
    const mockImg3 = {} as CanvasImageSource;

    const baseFrames = [
      { timestampMs: 0, image: mockImg1 },
      { timestampMs: 500, image: mockImg2 },
      { timestampMs: 1000, image: mockImg3 },
    ];

    expect(findMatchingBaseFrame([], 250)).toBeNull();
    expect(findMatchingBaseFrame(undefined, 250)).toBeNull();
    expect(findMatchingBaseFrame(baseFrames, 0)).toBe(mockImg1);
    expect(findMatchingBaseFrame(baseFrames, 100)).toBe(mockImg1);
    expect(findMatchingBaseFrame(baseFrames, 400)).toBe(mockImg2);
    expect(findMatchingBaseFrame(baseFrames, 800)).toBe(mockImg3);
    expect(findMatchingBaseFrame(baseFrames, 1200)).toBe(mockImg3);
  });

  it('compositeTimelineFrames renders base frames via drawImage', async () => {
    const ctx = createMockContext();
    const mockBaseImage = { width: 100, height: 100 } as CanvasImageSource;

    await compositeTimelineFrames({
      trimStartMs: 0,
      trimEndMs: 200,
      fps: 10,
      baseFrames: [{ timestampMs: 0, image: mockBaseImage }],
      contextProvider: () => ctx,
    });

    expect(ctx.drawImage).toHaveBeenCalledWith(mockBaseImage, 0, 0, 960, 540);
  });

  it('compositeTimelineFrames renders base frames via putImageData when ImageData provided', async () => {
    const ctx = createMockContext();
    const mockImageData = {
      data: new Uint8ClampedArray(100 * 100 * 4),
      width: 100,
      height: 100,
    } as unknown as ImageData;

    await compositeTimelineFrames({
      trimStartMs: 0,
      trimEndMs: 100,
      fps: 10,
      baseFrames: [{ timestampMs: 0, image: mockImageData }],
      contextProvider: () => ctx,
    });

    expect(ctx.putImageData).toHaveBeenCalledWith(mockImageData, 0, 0);
  });

  it('compositeTimelineFrames invokes getBaseFrame callback for each sample', async () => {
    const ctx = createMockContext();
    const mockImage = {} as CanvasImageSource;
    const getBaseFrame = vi.fn().mockReturnValue(mockImage);

    await compositeTimelineFrames({
      trimStartMs: 0,
      trimEndMs: 200,
      fps: 10,
      getBaseFrame,
      contextProvider: () => ctx,
    });

    expect(getBaseFrame).toHaveBeenCalledTimes(3);
    expect(ctx.drawImage).toHaveBeenCalledWith(mockImage, 0, 0, 960, 540);
  });
});

describe('exportCompositor: Cancellation Support', () => {
  it('ExportCompositor.cancel() stops frame composition early', async () => {
    const compositor = new ExportCompositor();
    let frameCounter = 0;

    await expect(
      compositor.export({
        trimStartMs: 0,
        trimEndMs: 1000,
        fps: 10, // 11 frames
        onProgress: () => {
          frameCounter++;
          if (frameCounter === 3) {
            compositor.cancel();
          }
        },
      })
    ).rejects.toThrow(ExportCancelledError);

    expect(compositor.cancelled).toBe(true);
    expect(frameCounter).toBeLessThan(11);
  });

  it('compositeTimelineFrames aborts when isCancelled returns true', async () => {
    let callCount = 0;
    await expect(
      compositeTimelineFrames({
        trimStartMs: 0,
        trimEndMs: 500,
        fps: 10,
        isCancelled: () => {
          callCount++;
          return callCount > 2;
        },
      })
    ).rejects.toThrow(ExportCancelledError);
  });

  it('exportRecordingToGif aborts when abortSignal is triggered', async () => {
    const controller = new AbortController();
    const onProgress = vi.fn();

    controller.abort();

    await expect(
      exportRecordingToGif({
        trimStartMs: 0,
        trimEndMs: 200,
        fps: 10,
        abortSignal: controller.signal,
        onProgress,
      })
    ).rejects.toThrow(ExportCancelledError);

    expect(onProgress).toHaveBeenCalledWith(
      expect.objectContaining({
        stage: 'error',
        errorMessage: expect.stringContaining('Export cancelled'),
      })
    );
  });
});

describe('exportCompositor: Native IPC Pipeline & Error Propagation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (globalThis as any).window = {
      __TAURI_INTERNALS__: {},
    };
  });

  afterEach(() => {
    delete (globalThis as any).window;
  });

  it('connects composited frames directly to encode_raw_frames IPC command', async () => {
    const mockInvoke = vi.mocked(invoke);
    mockInvoke.mockResolvedValueOnce('mock_screencast.gif');

    const result = await exportRecordingToGif({
      outputPath: 'test.gif',
      trimStartMs: 0,
      trimEndMs: 200,
      fps: 10,
      quality: 85,
    });

    expect(result.success).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith(
      'encode_raw_frames',
      expect.objectContaining({
        outputPath: 'test.gif',
        options: expect.objectContaining({
          fps: 10,
          quality: 85,
        }),
        frames: expect.any(Array),
      })
    );

    const callArgs = mockInvoke.mock.calls[0][1] as any;
    expect(callArgs.frames.length).toBe(3);
    expect(callArgs.frames[0]).toHaveProperty('timestampMs', 0);
    expect(callArgs.frames[0]).toHaveProperty('rgbaData');
    expect(Array.isArray(callArgs.frames[0].rgbaData)).toBe(true);
  });

  it('invokes export_gif when useSessionCache is true', async () => {
    const mockInvoke = vi.mocked(invoke);
    mockInvoke.mockResolvedValueOnce('session_screencast.gif');

    const result = await exportSessionToGif({
      outputPath: 'session.gif',
      trimStartMs: 0,
      trimEndMs: 300,
      fps: 15,
      quality: 90,
      elements: [
        {
          id: 'rect_1',
          tool: 'rectangle',
          start_ms: 0,
          end_ms: 300,
          x: 10,
          y: 10,
          width: 50,
          height: 50,
        },
      ],
    });

    expect(result.success).toBe(true);
    expect(mockInvoke).toHaveBeenCalledWith(
      'update_vector_ledger',
      expect.anything()
    );
    expect(mockInvoke).toHaveBeenCalledWith(
      'export_gif',
      expect.objectContaining({
        outputPath: 'session.gif',
        options: expect.objectContaining({
          fps: 15,
          quality: 90,
        }),
        trimStartMs: 0,
        trimEndMs: 300,
      })
    );
  });

  it('propagates IPC error without swallowing and reports stage error', async () => {
    const mockInvoke = vi.mocked(invoke);
    mockInvoke.mockRejectedValueOnce(new Error('Encoder crashed: out of disk space'));

    const onProgress = vi.fn();

    await expect(
      exportRecordingToGif({
        outputPath: 'fail.gif',
        trimStartMs: 0,
        trimEndMs: 100,
        fps: 10,
        onProgress,
      })
    ).rejects.toThrow('Encoder crashed: out of disk space');

    // Confirm that stage: 'error' was emitted with actual error message
    expect(onProgress).toHaveBeenCalledWith(
      expect.objectContaining({
        stage: 'error',
        errorMessage: 'Encoder crashed: out of disk space',
      })
    );
  });

  it('propagates export_gif error when useSessionCache fails', async () => {
    const mockInvoke = vi.mocked(invoke);
    mockInvoke.mockRejectedValueOnce(new Error('No recorded frames in cache'));

    const onProgress = vi.fn();

    await expect(
      exportSessionToGif({
        outputPath: 'fail_session.gif',
        trimStartMs: 0,
        trimEndMs: 100,
        fps: 10,
        onProgress,
      })
    ).rejects.toThrow('No recorded frames in cache');

    expect(onProgress).toHaveBeenCalledWith(
      expect.objectContaining({
        stage: 'error',
        errorMessage: 'No recorded frames in cache',
      })
    );
  });
});
