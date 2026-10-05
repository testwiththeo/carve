import React from 'react';
import { Group, Rect, Ellipse, Line } from 'react-konva';
import type { KonvaEventObject } from 'konva/lib/Node';
import { VectorElement } from '../../../types/vector';

export interface ShapeToolProps {
  element: VectorElement;
  isSelected?: boolean;
  onSelect?: (element: VectorElement) => void;
  onChange?: (patch: Partial<VectorElement>) => void;
  readOnly?: boolean;
}

export const ShapeTool: React.FC<ShapeToolProps> = ({
  element,
  isSelected = false,
  onSelect,
  onChange,
  readOnly = false,
}) => {
  const { tool, points = [], stroke = '#EF4444', strokeWidth = 3, fill = 'transparent' } = element;
  const x = element.x ?? 0;
  const y = element.y ?? 0;
  const width = element.width ?? 0;
  const height = element.height ?? 0;

  const handleGroupDragEnd = (e: KonvaEventObject<DragEvent>) => {
    if (tool === 'pencil' || tool === 'highlight') {
      const dx = e.target.x();
      const dy = e.target.y();
      e.target.position({ x: 0, y: 0 });
      const updatedPoints = points.map((val, idx) =>
        idx % 2 === 0 ? Math.round(val + dx) : Math.round(val + dy)
      );
      onChange?.({ points: updatedPoints });
    } else {
      onChange?.({
        x: Math.round(e.target.x()),
        y: Math.round(e.target.y()),
      });
    }
  };

  if (tool === 'rectangle') {
    return (
      <Group
        id={element.id}
        x={x}
        y={y}
        draggable={!readOnly && isSelected}
        onClick={() => onSelect?.(element)}
        onTap={() => onSelect?.(element)}
        onDragEnd={handleGroupDragEnd}
      >
        {isSelected && (
          <Rect
            x={-2}
            y={-2}
            width={width + 4}
            height={height + 4}
            stroke="#3B82F6"
            strokeWidth={1.5}
            dash={[4, 4]}
            cornerRadius={6}
          />
        )}
        <Rect
          width={width}
          height={height}
          stroke={stroke}
          strokeWidth={strokeWidth}
          fill={fill}
          cornerRadius={4}
          shadowColor="rgba(0, 0, 0, 0.25)"
          shadowBlur={isSelected ? 4 : 2}
        />
      </Group>
    );
  }

  if (tool === 'ellipse') {
    const rx = Math.abs(width / 2);
    const ry = Math.abs(height / 2);
    const cx = x + rx;
    const cy = y + ry;

    return (
      <Group
        id={element.id}
        draggable={!readOnly && isSelected}
        onClick={() => onSelect?.(element)}
        onTap={() => onSelect?.(element)}
        onDragEnd={(e) => {
          const dx = e.target.x();
          const dy = e.target.y();
          e.target.position({ x: 0, y: 0 });
          onChange?.({
            x: Math.round(x + dx),
            y: Math.round(y + dy),
          });
        }}
      >
        {isSelected && (
          <Ellipse
            x={cx}
            y={cy}
            radiusX={rx + 3}
            radiusY={ry + 3}
            stroke="#3B82F6"
            strokeWidth={1.5}
            dash={[4, 4]}
          />
        )}
        <Ellipse
          x={cx}
          y={cy}
          radiusX={rx}
          radiusY={ry}
          stroke={stroke}
          strokeWidth={strokeWidth}
          fill={fill}
          shadowColor="rgba(0, 0, 0, 0.25)"
          shadowBlur={isSelected ? 4 : 2}
        />
      </Group>
    );
  }

  if (tool === 'pencil') {
    return (
      <Group
        id={element.id}
        draggable={!readOnly && isSelected}
        onClick={() => onSelect?.(element)}
        onTap={() => onSelect?.(element)}
        onDragEnd={handleGroupDragEnd}
      >
        {/* Wider hit zone */}
        <Line
          points={points}
          stroke="transparent"
          strokeWidth={Math.max(14, strokeWidth + 10)}
          lineCap="round"
          lineJoin="round"
        />
        {isSelected && (
          <Line
            points={points}
            stroke="#3B82F6"
            strokeWidth={strokeWidth + 4}
            opacity={0.5}
            lineCap="round"
            lineJoin="round"
          />
        )}
        <Line
          points={points}
          stroke={stroke}
          strokeWidth={strokeWidth}
          tension={0.4}
          lineCap="round"
          lineJoin="round"
        />
      </Group>
    );
  }

  if (tool === 'highlight') {
    const highlightWidth = Math.max(16, strokeWidth * 4);
    return (
      <Group
        id={element.id}
        draggable={!readOnly && isSelected}
        onClick={() => onSelect?.(element)}
        onTap={() => onSelect?.(element)}
        onDragEnd={handleGroupDragEnd}
      >
        <Line
          points={points}
          stroke={stroke || '#FBBF24'}
          strokeWidth={highlightWidth}
          opacity={0.35}
          lineCap="round"
          lineJoin="round"
        />
        {isSelected && (
          <Line
            points={points}
            stroke="#3B82F6"
            strokeWidth={highlightWidth + 4}
            opacity={0.3}
            lineCap="round"
            lineJoin="round"
          />
        )}
      </Group>
    );
  }

  return null;
};

export default ShapeTool;
