import React from 'react';
import { Group, Rect, Shape } from 'react-konva';
import type { KonvaEventObject } from 'konva/lib/Node';
import { VectorElement } from '../../../types/vector';

export interface BlurToolProps {
  element: VectorElement;
  isSelected?: boolean;
  onSelect?: (element: VectorElement) => void;
  onChange?: (patch: Partial<VectorElement>) => void;
  readOnly?: boolean;
}

/**
 * Renders a pixelated/blurred bounding box rectangle.
 * Uses an offscreen tile computation / mosaic sceneFunc to achieve pixelation
 * over the specified canvas region with crisp boundaries.
 */
export const BlurTool: React.FC<BlurToolProps> = ({
  element,
  isSelected = false,
  onSelect,
  onChange,
  readOnly = false,
}) => {
  const x = element.x ?? 0;
  const y = element.y ?? 0;
  const width = Math.max(10, element.width ?? 120);
  const height = Math.max(10, element.height ?? 80);
  const blockSize = 10;

  const handleDragEnd = (e: KonvaEventObject<DragEvent>) => {
    onChange?.({
      x: Math.round(e.target.x()),
      y: Math.round(e.target.y()),
    });
  };

  return (
    <Group
      id={element.id}
      x={x}
      y={y}
      draggable={!readOnly && isSelected}
      onClick={() => onSelect?.(element)}
      onTap={() => onSelect?.(element)}
      onDragEnd={handleDragEnd}
    >
      {/* Pixelated mosaic pattern */}
      <Shape
        width={width}
        height={height}
        sceneFunc={(ctx) => {
          const cols = Math.ceil(width / blockSize);
          const rows = Math.ceil(height / blockSize);

          for (let r = 0; r < rows; r++) {
            for (let c = 0; c < cols; c++) {
              const bx = c * blockSize;
              const by = r * blockSize;
              const bw = Math.min(blockSize, width - bx);
              const bh = Math.min(blockSize, height - by);

              // Procedural deterministic mosaic blocks
              const seed = ((c * 374761393 + r * 668265263 + 1013904223) ^ 0x5bf03635) >>> 0;
              const lightness = 130 + (seed % 65);
              const alpha = 0.88;

              ctx.fillStyle = `rgba(${lightness}, ${lightness}, ${lightness + 8}, ${alpha})`;
              ctx.fillRect(bx, by, bw, bh);
            }
          }

          // Subtle mosaic cell grid lines for realistic pixelation look
          ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
          ctx.lineWidth = 1;
          for (let c = 0; c <= cols; c++) {
            const gx = Math.min(c * blockSize, width);
            ctx.beginPath();
            ctx.moveTo(gx, 0);
            ctx.lineTo(gx, height);
            ctx.stroke();
          }
          for (let r = 0; r <= rows; r++) {
            const gy = Math.min(r * blockSize, height);
            ctx.beginPath();
            ctx.moveTo(0, gy);
            ctx.lineTo(width, gy);
            ctx.stroke();
          }
        }}
      />

      {/* Bounding box outline */}
      <Rect
        width={width}
        height={height}
        stroke={isSelected ? '#3B82F6' : '#64748B'}
        strokeWidth={isSelected ? 2 : 1.5}
        dash={isSelected ? undefined : [5, 4]}
        cornerRadius={3}
      />
    </Group>
  );
};

export default BlurTool;
