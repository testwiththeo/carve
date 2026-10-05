import { create } from 'zustand';
import {
  VectorElement,
  VectorToolKind,
  isVectorElementActiveAt,
} from '../types/vector';

export interface AnnotationStoreState {
  activeTool: VectorToolKind;
  strokeColor: string;
  strokeWidth: number;
  fillColor: string;
  fontSize: number;
  fontFamily: string;
  elements: VectorElement[];
  selectedElementId: string | null;
  activeTimestampMs: number;
  isDrawing: boolean;
  drawingElement: VectorElement | null;

  // Actions
  setActiveTool: (tool: VectorToolKind) => void;
  setStrokeColor: (color: string) => void;
  setStrokeWidth: (width: number) => void;
  setFillColor: (color: string) => void;
  setFontSize: (size: number) => void;
  setActiveTimestampMs: (ms: number) => void;
  setIsDrawing: (isDrawing: boolean) => void;
  setDrawingElement: (element: VectorElement | null) => void;

  addElement: (element: Partial<VectorElement> & { tool: VectorToolKind }) => VectorElement;
  updateElement: (id: string, patch: Partial<VectorElement>) => void;
  removeElement: (id: string) => void;
  clearElements: () => void;
  selectElement: (id: string | null) => void;
  setElements: (elements: VectorElement[]) => void;

  // Selectors / Helpers
  getActiveElements: (timestampMs?: number) => VectorElement[];
  getNextCounterValue: () => number;
  generateId: (prefix?: string) => string;
}

let nextIdCounter = 0;

export function generateElementId(prefix = 'elem'): string {
  nextIdCounter += 1;
  return `${prefix}_${Date.now()}_${nextIdCounter}`;
}

export const useAnnotationStore = create<AnnotationStoreState>((set, get) => ({
  activeTool: 'arrow',
  strokeColor: '#FF0000',
  strokeWidth: 3,
  fillColor: 'transparent',
  fontSize: 18,
  fontFamily: 'system-ui, sans-serif',
  elements: [],
  selectedElementId: null,
  activeTimestampMs: 0,
  isDrawing: false,
  drawingElement: null,

  setActiveTool: (activeTool: VectorToolKind) => set({ activeTool }),
  setStrokeColor: (strokeColor: string) => set({ strokeColor }),
  setStrokeWidth: (strokeWidth: number) => set({ strokeWidth }),
  setFillColor: (fillColor: string) => set({ fillColor }),
  setFontSize: (fontSize: number) => set({ fontSize }),
  setActiveTimestampMs: (activeTimestampMs: number) => set({ activeTimestampMs }),
  setIsDrawing: (isDrawing: boolean) => set({ isDrawing }),
  setDrawingElement: (drawingElement: VectorElement | null) => set({ drawingElement }),

  addElement: (elementInput) => {
    const state = get();
    const id = elementInput.id || generateElementId(elementInput.tool);
    const start_ms = elementInput.start_ms ?? state.activeTimestampMs;
    const end_ms = elementInput.end_ms ?? (start_ms + 3000);
    const isBadge = elementInput.tool === 'counter_badge' || elementInput.tool === 'counterBadge';
    const count = isBadge ? (elementInput.count ?? state.getNextCounterValue()) : elementInput.count;

    const newElement: VectorElement = {
      id,
      tool: elementInput.tool,
      start_ms,
      end_ms,
      data: elementInput.data ?? {},
      stroke: elementInput.stroke ?? state.strokeColor,
      strokeWidth: elementInput.strokeWidth ?? state.strokeWidth,
      fill: elementInput.fill ?? (isBadge ? (elementInput.stroke ?? state.strokeColor) : state.fillColor),
      points: elementInput.points,
      text: elementInput.text,
      fontSize: elementInput.fontSize ?? state.fontSize,
      fontFamily: elementInput.fontFamily ?? state.fontFamily,
      count,
      x: elementInput.x,
      y: elementInput.y,
      width: elementInput.width,
      height: elementInput.height,
    };

    set((current) => ({
      elements: [...current.elements, newElement],
    }));

    return newElement;
  },

  updateElement: (id: string, patch: Partial<VectorElement>) => {
    set((state) => ({
      elements: state.elements.map((el) => {
        if (el.id !== id) return el;
        return {
          ...el,
          ...patch,
          data: patch.data ? { ...el.data, ...patch.data } : el.data,
        };
      }),
    }));
  },

  removeElement: (id: string) => {
    set((state) => ({
      elements: state.elements.filter((el) => el.id !== id),
      selectedElementId: state.selectedElementId === id ? null : state.selectedElementId,
    }));
  },

  clearElements: () => {
    set({
      elements: [],
      selectedElementId: null,
      drawingElement: null,
      isDrawing: false,
    });
  },

  selectElement: (id: string | null) => {
    set({ selectedElementId: id });
  },

  setElements: (elements: VectorElement[]) => {
    set({ elements: [...elements] });
  },

  getActiveElements: (timestampMs?: number) => {
    const ts = timestampMs ?? get().activeTimestampMs;
    return get().elements.filter((el) => isVectorElementActiveAt(el, ts));
  },

  getNextCounterValue: () => {
    const elements = get().elements;
    let max = 0;
    for (const el of elements) {
      if (el.tool === 'counter_badge' || el.tool === 'counterBadge') {
        const c = typeof el.count === 'number' ? el.count : 0;
        if (c > max) max = c;
      }
    }
    return max + 1;
  },

  generateId: (prefix = 'elem') => generateElementId(prefix),
}));
