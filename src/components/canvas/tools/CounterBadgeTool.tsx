import React from 'react';
import { Group, Circle, Text, Ring } from 'react-konva';
import type { KonvaEventObject } from 'konva/lib/Node';
import { VectorElement } from '../../../types/vector';

export interface CounterBadgeToolProps {
  element: VectorElement;
  isSelected?: boolean;
  onSelect?: (element: VectorElement) => void;
  onChange?: (patch: Partial<VectorElement>) => void;
  readOnly?: boolean;
}

export const CounterBadgeTool: React.FC<CounterBadgeToolProps> = ({
  element,
  isSelected = false,
  onSelect,
  onChange,
  readOnly = false,
}) => {
  const x = element.x ?? 0;
  const y = element.y ?? 0;
  const count = element.count ?? 1;
  const countStr = String(count);
  const badgeColor = element.fill && element.fill !== 'transparent'
    ? element.fill
    : (element.stroke ?? '#EF4444');
  const radius = Math.max(14, 10 + countStr.length * 4);
  const fontSize = Math.max(12, Math.round(radius * 0.9));

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
      {/* Selection indicator ring */}
      {isSelected && (
        <Ring
          innerRadius={radius + 3}
          outerRadius={radius + 5}
          fill="#3B82F6"
          opacity={0.8}
        />
      )}

      {/* Main badge circle */}
      <Circle
        radius={radius}
        fill={badgeColor}
        stroke="#FFFFFF"
        strokeWidth={2}
        shadowColor="rgba(0, 0, 0, 0.4)"
        shadowBlur={6}
        shadowOffsetY={2}
      />

      {/* Badge number text */}
      <Text
        text={countStr}
        fontSize={fontSize}
        fontFamily="system-ui, -apple-system, sans-serif"
        fontStyle="bold"
        fill="#FFFFFF"
        align="center"
        verticalAlign="middle"
        width={radius * 2}
        height={radius * 2}
        offsetX={radius}
        offsetY={radius}
        listening={false}
      />
    </Group>
  );
};

export default CounterBadgeTool;
