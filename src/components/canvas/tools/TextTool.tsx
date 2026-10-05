import React, { useRef, useState } from 'react';
import { Group, Rect, Text } from 'react-konva';
import type { KonvaEventObject } from 'konva/lib/Node';
import type Konva from 'konva';
import { VectorElement } from '../../../types/vector';

export interface TextToolProps {
  element: VectorElement;
  isSelected?: boolean;
  onSelect?: (element: VectorElement) => void;
  onChange?: (patch: Partial<VectorElement>) => void;
  readOnly?: boolean;
}

export const TextTool: React.FC<TextToolProps> = ({
  element,
  isSelected = false,
  onSelect,
  onChange,
  readOnly = false,
}) => {
  const textRef = useRef<Konva.Text>(null);
  const [isEditing, setIsEditing] = useState(false);

  const x = element.x ?? 0;
  const y = element.y ?? 0;
  const textValue = element.text ?? 'Text';
  const fontSize = element.fontSize ?? 16;
  const fontFamily = element.fontFamily ?? 'system-ui, -apple-system, sans-serif';
  const strokeColor = element.stroke ?? '#FFFFFF';
  const pillFill = element.fill && element.fill !== 'transparent'
    ? element.fill
    : 'rgba(15, 23, 42, 0.88)';

  // Approximate width & height if not measured yet
  const paddingX = 14;
  const paddingY = 8;
  const approxTextWidth = Math.max(20, textValue.length * (fontSize * 0.58));
  const pillWidth = Math.max(element.width ?? 0, approxTextWidth + paddingX * 2);
  const pillHeight = Math.max(element.height ?? 0, fontSize + paddingY * 2);

  const handleDragEnd = (e: KonvaEventObject<DragEvent>) => {
    onChange?.({
      x: Math.round(e.target.x()),
      y: Math.round(e.target.y()),
    });
  };

  const handleDblClick = () => {
    if (readOnly) return;
    setIsEditing(true);

    const node = textRef.current;
    if (!node) return;

    const stage = node.getStage();
    if (!stage) return;

    const container = stage.container();
    const stageBox = container.getBoundingClientRect();
    const textPosition = node.getAbsolutePosition();

    const area = document.createElement('input');
    area.type = 'text';
    area.value = textValue;
    area.style.position = 'fixed';
    area.style.top = `${stageBox.top + textPosition.y - 4}px`;
    area.style.left = `${stageBox.left + textPosition.x - 4}px`;
    area.style.width = `${Math.max(120, pillWidth)}px`;
    area.style.height = `${pillHeight + 4}px`;
    area.style.fontSize = `${fontSize}px`;
    area.style.fontFamily = fontFamily;
    area.style.border = '2px solid #3B82F6';
    area.style.padding = '4px 8px';
    area.style.borderRadius = '8px';
    area.style.color = '#FFFFFF';
    area.style.background = '#0F172A';
    area.style.outline = 'none';
    area.style.zIndex = '10000';

    document.body.appendChild(area);
    area.focus();
    area.select();

    const finishEdit = () => {
      const updated = area.value.trim() || 'Text';
      onChange?.({ text: updated });
      setIsEditing(false);
      if (document.body.contains(area)) {
        document.body.removeChild(area);
      }
    };

    area.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        finishEdit();
      } else if (e.key === 'Escape') {
        setIsEditing(false);
        if (document.body.contains(area)) {
          document.body.removeChild(area);
        }
      }
    });

    area.addEventListener('blur', () => {
      finishEdit();
    });
  };

  return (
    <Group
      id={element.id}
      x={x}
      y={y}
      draggable={!readOnly && isSelected && !isEditing}
      onClick={() => onSelect?.(element)}
      onTap={() => onSelect?.(element)}
      onDblClick={handleDblClick}
      onDblTap={handleDblClick}
      onDragEnd={handleDragEnd}
    >
      {/* Background pill */}
      <Rect
        width={pillWidth}
        height={pillHeight}
        cornerRadius={pillHeight / 2}
        fill={pillFill}
        stroke={isSelected ? '#3B82F6' : 'rgba(255, 255, 255, 0.15)'}
        strokeWidth={isSelected ? 2 : 1}
        shadowColor="rgba(0, 0, 0, 0.35)"
        shadowBlur={8}
        shadowOffsetY={2}
      />

      {/* Text label inside pill */}
      <Text
        ref={textRef}
        x={paddingX}
        y={paddingY}
        text={textValue}
        fontSize={fontSize}
        fontFamily={fontFamily}
        fill={strokeColor}
        listening={false}
      />
    </Group>
  );
};

export default TextTool;
