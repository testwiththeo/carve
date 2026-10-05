import React, { useRef, useState, useCallback, useMemo } from 'react';
import { Stage, Layer } from 'react-konva';
import type Konva from 'konva';
import { VectorElement } from '../../types/vector';
import { useAnnotationStore } from '../../stores/annotationStore';
import { ArrowTool } from './tools/ArrowTool';
import { CounterBadgeTool } from './tools/CounterBadgeTool';
import { BlurTool } from './tools/BlurTool';
import { TextTool } from './tools/TextTool';
import { ShapeTool } from './tools/ShapeTool';

export interface VectorStageProps {
  width?: number;
  height?: number;
  elements?: VectorElement[];
  activeTimestampMs?: number;
  onDrawEnd?: (element: VectorElement) => void;
  onSelectElement?: (element: VectorElement | null) => void;
  onUpdateElement?: (id: string, patch: Partial<VectorElement>) => void;
  readOnly?: boolean;
  className?: string;
  style?: React.CSSProperties;
}

interface DrawingState {
  isDrawing: boolean;
  startPoint: { x: number; y: number } | null;
  element: VectorElement | null;
}

export const VectorStage: React.FC<VectorStageProps> = ({
  width = 800,
  height = 600,
  elements: propsElements,
  activeTimestampMs,
  onDrawEnd,
  onSelectElement,
  onUpdateElement,
  readOnly = false,
  className,
  style,
}) => {
  const stageRef = useRef<Konva.Stage>(null);

  const {
    activeTool,
    strokeColor,
    strokeWidth,
    fillColor,
    fontSize,
    fontFamily,
    elements: storeElements,
    selectedElementId,
    selectElement,
    addElement,
    updateElement: storeUpdateElement,
    getNextCounterValue,
    getActiveElements,
  } = useAnnotationStore();

  const [drawingState, setDrawingState] = useState<DrawingState>({
    isDrawing: false,
    startPoint: null,
    element: null,
  });

  // Calculate elements to display based on props or store
  const displayElements = useMemo(() => {
    if (propsElements) {
      if (activeTimestampMs !== undefined) {
        return propsElements.filter(
          (el) => activeTimestampMs >= el.start_ms && activeTimestampMs <= el.end_ms
        );
      }
      return propsElements;
    }
    return getActiveElements(activeTimestampMs);
  }, [propsElements, activeTimestampMs, getActiveElements, storeElements]);

  const handleSelect = useCallback(
    (element: VectorElement | null) => {
      selectElement(element ? element.id : null);
      onSelectElement?.(element);
    },
    [selectElement, onSelectElement]
  );

  const handleElementChange = useCallback(
    (id: string, patch: Partial<VectorElement>) => {
      storeUpdateElement(id, patch);
      onUpdateElement?.(id, patch);
    },
    [storeUpdateElement, onUpdateElement]
  );

  const handleMouseDown = useCallback(
    (e: Konva.KonvaEventObject<MouseEvent | TouchEvent>) => {
      if (readOnly) return;

      const stage = e.target.getStage();
      if (!stage) return;

      // Only trigger drawing when clicking directly on empty stage background
      const clickedOnEmpty = e.target === stage;
      if (!clickedOnEmpty) return;

      const pos = stage.getPointerPosition();
      if (!pos) return;

      const startMs = activeTimestampMs ?? 0;
      const endMs = startMs + 3000;

      // Handle single-click placement tools
      if (activeTool === 'counter_badge' || activeTool === 'counterBadge') {
        const badgeCount = getNextCounterValue();
        const created = addElement({
          tool: 'counter_badge',
          x: Math.round(pos.x),
          y: Math.round(pos.y),
          count: badgeCount,
          stroke: strokeColor,
          start_ms: startMs,
          end_ms: endMs,
        });
        handleSelect(created);
        onDrawEnd?.(created);
        return;
      }

      if (activeTool === 'text') {
        const created = addElement({
          tool: 'text',
          x: Math.round(pos.x),
          y: Math.round(pos.y),
          text: 'Text',
          fontSize,
          fontFamily,
          stroke: strokeColor,
          start_ms: startMs,
          end_ms: endMs,
        });
        handleSelect(created);
        onDrawEnd?.(created);
        return;
      }

      // Handle drag-based drawing tools
      let initialElement: VectorElement | null = null;
      const x = Math.round(pos.x);
      const y = Math.round(pos.y);

      switch (activeTool) {
        case 'arrow':
          initialElement = {
            id: 'drawing_temp',
            tool: 'arrow',
            points: [x, y, x, y],
            stroke: strokeColor,
            strokeWidth,
            start_ms: startMs,
            end_ms: endMs,
            data: {},
          };
          break;
        case 'rectangle':
          initialElement = {
            id: 'drawing_temp',
            tool: 'rectangle',
            x,
            y,
            width: 0,
            height: 0,
            stroke: strokeColor,
            strokeWidth,
            fill: fillColor,
            start_ms: startMs,
            end_ms: endMs,
            data: {},
          };
          break;
        case 'ellipse':
          initialElement = {
            id: 'drawing_temp',
            tool: 'ellipse',
            x,
            y,
            width: 0,
            height: 0,
            stroke: strokeColor,
            strokeWidth,
            fill: fillColor,
            start_ms: startMs,
            end_ms: endMs,
            data: {},
          };
          break;
        case 'blur':
          initialElement = {
            id: 'drawing_temp',
            tool: 'blur',
            x,
            y,
            width: 0,
            height: 0,
            start_ms: startMs,
            end_ms: endMs,
            data: {},
          };
          break;
        case 'pencil':
          initialElement = {
            id: 'drawing_temp',
            tool: 'pencil',
            points: [x, y],
            stroke: strokeColor,
            strokeWidth,
            start_ms: startMs,
            end_ms: endMs,
            data: {},
          };
          break;
        case 'highlight':
          initialElement = {
            id: 'drawing_temp',
            tool: 'highlight',
            points: [x, y],
            stroke: strokeColor || '#FBBF24',
            strokeWidth,
            start_ms: startMs,
            end_ms: endMs,
            data: {},
          };
          break;
        default:
          // Selection or unhandled tool: deselect
          handleSelect(null);
          return;
      }

      if (initialElement) {
        setDrawingState({
          isDrawing: true,
          startPoint: { x, y },
          element: initialElement,
        });
      }
    },
    [
      readOnly,
      activeTool,
      activeTimestampMs,
      getNextCounterValue,
      addElement,
      strokeColor,
      strokeWidth,
      fillColor,
      fontSize,
      fontFamily,
      handleSelect,
      onDrawEnd,
    ]
  );

  const handleMouseMove = useCallback(
    (e: Konva.KonvaEventObject<MouseEvent | TouchEvent>) => {
      if (!drawingState.isDrawing || !drawingState.startPoint || !drawingState.element) return;

      const stage = e.target.getStage();
      if (!stage) return;

      const pos = stage.getPointerPosition();
      if (!pos) return;

      const currentX = Math.round(pos.x);
      const currentY = Math.round(pos.y);
      const { startPoint, element } = drawingState;

      let updatedElement: VectorElement = { ...element };

      switch (element.tool) {
        case 'arrow':
          updatedElement = {
            ...updatedElement,
            points: [startPoint.x, startPoint.y, currentX, currentY],
          };
          break;
        case 'rectangle':
        case 'ellipse':
        case 'blur': {
          const rx = Math.min(startPoint.x, currentX);
          const ry = Math.min(startPoint.y, currentY);
          const rw = Math.abs(currentX - startPoint.x);
          const rh = Math.abs(currentY - startPoint.y);
          updatedElement = {
            ...updatedElement,
            x: rx,
            y: ry,
            width: rw,
            height: rh,
          };
          break;
        }
        case 'pencil':
        case 'highlight':
          updatedElement = {
            ...updatedElement,
            points: [...(element.points ?? []), currentX, currentY],
          };
          break;
      }

      setDrawingState((prev) => ({
        ...prev,
        element: updatedElement,
      }));
    },
    [drawingState]
  );

  const handleMouseUp = useCallback(() => {
    if (!drawingState.isDrawing || !drawingState.element || !drawingState.startPoint) {
      setDrawingState({ isDrawing: false, startPoint: null, element: null });
      return;
    }

    const { element } = drawingState;
    let isValid = false;

    // Minimum size validation
    switch (element.tool) {
      case 'arrow': {
        const [x1, y1, x2, y2] = element.points ?? [0, 0, 0, 0];
        const dist = Math.hypot(x2 - x1, y2 - y1);
        isValid = dist >= 6;
        break;
      }
      case 'rectangle':
      case 'ellipse':
      case 'blur':
        isValid = (element.width ?? 0) >= 6 && (element.height ?? 0) >= 6;
        break;
      case 'pencil':
      case 'highlight':
        isValid = (element.points?.length ?? 0) >= 4;
        break;
      default:
        isValid = true;
    }

    if (isValid) {
      // Remove temporary ID and commit to store
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { id: _tempId, ...elemToSave } = element;
      const created = addElement(elemToSave);
      handleSelect(created);
      onDrawEnd?.(created);
    }

    setDrawingState({
      isDrawing: false,
      startPoint: null,
      element: null,
    });
  }, [drawingState, addElement, handleSelect, onDrawEnd]);

  const renderToolElement = (el: VectorElement, isSelected: boolean) => {
    const isTemp = el.id === 'drawing_temp';
    const commonProps = {
      key: el.id,
      element: el,
      isSelected: !isTemp && isSelected,
      onSelect: isTemp ? undefined : () => handleSelect(el),
      onChange: isTemp ? undefined : (patch: Partial<VectorElement>) => handleElementChange(el.id, patch),
      readOnly: readOnly || isTemp,
    };

    switch (el.tool) {
      case 'arrow':
        return <ArrowTool {...commonProps} />;
      case 'counter_badge':
      case 'counterBadge':
        return <CounterBadgeTool {...commonProps} />;
      case 'blur':
        return <BlurTool {...commonProps} />;
      case 'text':
        return <TextTool {...commonProps} />;
      case 'rectangle':
      case 'ellipse':
      case 'pencil':
      case 'highlight':
        return <ShapeTool {...commonProps} />;
      default:
        return null;
    }
  };

  return (
    <div className={`relative inline-block select-none ${className ?? ''}`} style={style}>
      <Stage
        ref={stageRef}
        width={width}
        height={height}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onTouchStart={handleMouseDown}
        onTouchMove={handleMouseMove}
        onTouchEnd={handleMouseUp}
      >
        <Layer>
          {displayElements.map((el) =>
            renderToolElement(el, el.id === selectedElementId)
          )}
          {drawingState.isDrawing && drawingState.element && (
            renderToolElement(drawingState.element, false)
          )}
        </Layer>
      </Stage>
    </div>
  );
};

export default VectorStage;
