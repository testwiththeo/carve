import React from 'react';
import { Arrow, Group } from 'react-konva';
import type { KonvaEventObject } from 'konva/lib/Node';
import { VectorElement } from '../../../types/vector';

export interface ArrowToolProps {
  element: VectorElement;
  isSelected?: boolean;
  onSelect?: (element: VectorElement) => void;
  onChange?: (patch: Partial<VectorElement>) => void;
  readOnly?: boolean;
}

export const ArrowTool: React.FC<ArrowToolProps> = ({
  element,
  isSelected = false,
  onSelect,
  onChange,
  readOnly = false,
}) => {
  const points = element.points ?? [0, 0, 100, 100];
  const stroke = element.stroke ?? '#EF4444';
  const strokeWidth = element.strokeWidth ?? 3;
  const pointerSize = Math.max(10, strokeWidth * 3.5);

  const handleDragEnd = (e: KonvaEventObject<DragEvent>) => {
    const dx = e.target.x();
    const dy = e.target.y();
    // Reset group/node position and offset points
    e.target.position({ x: 0, y: 0 });

    const updatedPoints = points.map((val, idx) =>
      idx % 2 === 0 ? Math.round(val + dx) : Math.round(val + dy)
    );

    onChange?.({ points: updatedPoints });
  };

  return (
    <Group
      id={element.id}
      draggable={!readOnly && isSelected}
      onClick={() => onSelect?.(element)}
      onTap={() => onSelect?.(element)}
      onDragEnd={handleDragEnd}
    >
      {/* Invisible wider stroke for easier clicking */}
      <Arrow
        points={points}
        stroke="transparent"
        strokeWidth={Math.max(16, strokeWidth + 12)}
        pointerLength={pointerSize + 4}
        pointerWidth={pointerSize + 4}
        lineCap="round"
        lineJoin="round"
      />
      {/* Selection shadow/halo */}
      {isSelected && (
        <Arrow
          points={points}
          stroke="#3B82F6"
          strokeWidth={strokeWidth + 4}
          pointerLength={pointerSize + 2}
          pointerWidth={pointerSize + 2}
          opacity={0.6}
          lineCap="round"
          lineJoin="round"
        />
      )}
      {/* Actual visible arrow */}
      <Arrow
        points={points}
        stroke={stroke}
        fill={stroke}
        strokeWidth={strokeWidth}
        pointerLength={pointerSize}
        pointerWidth={pointerSize}
        lineCap="round"
        lineJoin="round"
        shadowColor="rgba(0, 0, 0, 0.3)"
        shadowBlur={isSelected ? 4 : 2}
        shadowOffsetY={1}
      />
    </Group>
  );
};

export default ArrowTool;
