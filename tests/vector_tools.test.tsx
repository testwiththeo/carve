import { describe, it, expect, beforeEach } from 'vitest';
import { useAnnotationStore } from '../src/stores/annotationStore';
import { VectorStage } from '../src/components/canvas/VectorStage';
import { ArrowTool } from '../src/components/canvas/tools/ArrowTool';
import { CounterBadgeTool } from '../src/components/canvas/tools/CounterBadgeTool';
import { BlurTool } from '../src/components/canvas/tools/BlurTool';
import { TextTool } from '../src/components/canvas/tools/TextTool';
import { ShapeTool } from '../src/components/canvas/tools/ShapeTool';

describe('annotationStore', () => {
  beforeEach(() => {
    useAnnotationStore.getState().clearElements();
    useAnnotationStore.setState({
      activeTool: 'arrow',
      strokeColor: '#FF0000',
      strokeWidth: 3,
      fillColor: 'transparent',
      selectedElementId: null,
      activeTimestampMs: 0,
    });
  });

  it('adds an arrow element with monotonic timestamp', () => {
    const store = useAnnotationStore.getState();
    store.addElement({
      id: 'test_1',
      tool: 'arrow',
      start_ms: 100,
      end_ms: 3000,
      points: [0, 0, 100, 100],
      stroke: '#FF0000',
    });
    expect(useAnnotationStore.getState().elements.length).toBe(1);
    expect(useAnnotationStore.getState().elements[0].id).toBe('test_1');
    expect(useAnnotationStore.getState().elements[0].tool).toBe('arrow');
  });

  it('updates an existing element', () => {
    const store = useAnnotationStore.getState();
    store.addElement({
      id: 'test_rect',
      tool: 'rectangle',
      start_ms: 200,
      end_ms: 1000,
      x: 10,
      y: 20,
      width: 50,
      height: 50,
      stroke: '#00FF00',
    });

    store.updateElement('test_rect', {
      width: 120,
      stroke: '#0000FF',
    });

    const updated = useAnnotationStore.getState().elements.find((e) => e.id === 'test_rect');
    expect(updated).toBeDefined();
    expect(updated?.width).toBe(120);
    expect(updated?.stroke).toBe('#0000FF');
    expect(updated?.height).toBe(50);
  });

  it('removes an element and deselects it if selected', () => {
    const store = useAnnotationStore.getState();
    store.addElement({
      id: 'elem_to_delete',
      tool: 'ellipse',
      start_ms: 0,
      end_ms: 500,
    });
    store.selectElement('elem_to_delete');
    expect(useAnnotationStore.getState().selectedElementId).toBe('elem_to_delete');

    store.removeElement('elem_to_delete');
    expect(useAnnotationStore.getState().elements.length).toBe(0);
    expect(useAnnotationStore.getState().selectedElementId).toBeNull();
  });

  it('clears all elements and resets selection', () => {
    const store = useAnnotationStore.getState();
    store.addElement({ id: 'e1', tool: 'pencil', start_ms: 0, end_ms: 100 });
    store.addElement({ id: 'e2', tool: 'text', start_ms: 50, end_ms: 200 });
    store.selectElement('e1');

    store.clearElements();
    expect(useAnnotationStore.getState().elements).toEqual([]);
    expect(useAnnotationStore.getState().selectedElementId).toBeNull();
  });

  it('manages tool state (activeTool, strokeColor, strokeWidth, fillColor, fontSize)', () => {
    const store = useAnnotationStore.getState();

    store.setActiveTool('counter_badge');
    expect(useAnnotationStore.getState().activeTool).toBe('counter_badge');

    store.setStrokeColor('#3B82F6');
    expect(useAnnotationStore.getState().strokeColor).toBe('#3B82F6');

    store.setStrokeWidth(5);
    expect(useAnnotationStore.getState().strokeWidth).toBe(5);

    store.setFillColor('rgba(255, 0, 0, 0.5)');
    expect(useAnnotationStore.getState().fillColor).toBe('rgba(255, 0, 0, 0.5)');

    store.setFontSize(24);
    expect(useAnnotationStore.getState().fontSize).toBe(24);
  });

  it('filters active elements at specific timestamps', () => {
    const store = useAnnotationStore.getState();
    store.addElement({ id: 'e1', tool: 'arrow', start_ms: 100, end_ms: 500 });
    store.addElement({ id: 'e2', tool: 'blur', start_ms: 400, end_ms: 800 });
    store.addElement({ id: 'e3', tool: 'text', start_ms: 900, end_ms: 1200 });

    const at50 = store.getActiveElements(50);
    expect(at50.map((e) => e.id)).toEqual([]);

    const at450 = store.getActiveElements(450);
    expect(at450.map((e) => e.id)).toEqual(['e1', 'e2']);

    const at1000 = store.getActiveElements(1000);
    expect(at1000.map((e) => e.id)).toEqual(['e3']);
  });

  it('handles auto-incrementing counter badge generation', () => {
    const store = useAnnotationStore.getState();
    const count1 = store.getNextCounterValue();
    expect(count1).toBe(1);

    store.addElement({
      id: 'badge_1',
      tool: 'counter_badge',
      start_ms: 0,
      end_ms: 1000,
      count: 1,
      x: 10,
      y: 10,
    });

    const count2 = store.getNextCounterValue();
    expect(count2).toBe(2);

    store.addElement({
      id: 'badge_2',
      tool: 'counter_badge',
      start_ms: 0,
      end_ms: 1000,
      x: 30,
      y: 30,
    });
    const badge2 = useAnnotationStore.getState().elements.find((e) => e.id === 'badge_2');
    expect(badge2?.count).toBe(2);

    const count3 = store.getNextCounterValue();
    expect(count3).toBe(3);
  });

  it('supports creating all vector tool types with proper defaults', () => {
    const store = useAnnotationStore.getState();

    const arrow = store.addElement({ tool: 'arrow', points: [0, 0, 50, 50] });
    expect(arrow.tool).toBe('arrow');
    expect(arrow.stroke).toBe('#FF0000');
    expect(arrow.strokeWidth).toBe(3);

    const rect = store.addElement({ tool: 'rectangle', x: 0, y: 0, width: 100, height: 60 });
    expect(rect.tool).toBe('rectangle');
    expect(rect.width).toBe(100);

    const ellipse = store.addElement({ tool: 'ellipse', x: 10, y: 10, width: 40, height: 40 });
    expect(ellipse.tool).toBe('ellipse');

    const blur = store.addElement({ tool: 'blur', x: 20, y: 20, width: 80, height: 50 });
    expect(blur.tool).toBe('blur');

    const text = store.addElement({ tool: 'text', text: 'Sample', x: 5, y: 5 });
    expect(text.tool).toBe('text');
    expect(text.text).toBe('Sample');

    const pencil = store.addElement({ tool: 'pencil', points: [1, 2, 3, 4] });
    expect(pencil.tool).toBe('pencil');

    const highlight = store.addElement({ tool: 'highlight', points: [1, 2, 3, 4] });
    expect(highlight.tool).toBe('highlight');

    expect(useAnnotationStore.getState().elements.length).toBe(7);
  });

  it('allows bulk setting elements via setElements', () => {
    const store = useAnnotationStore.getState();
    store.setElements([
      { id: 'b1', tool: 'rectangle', start_ms: 0, end_ms: 1000, data: {} },
      { id: 'b2', tool: 'arrow', start_ms: 100, end_ms: 2000, data: {} },
    ]);
    expect(useAnnotationStore.getState().elements.length).toBe(2);
  });
});

describe('canvas components exports', () => {
  it('exports VectorStage and tool renderers', () => {
    expect(VectorStage).toBeDefined();
    expect(ArrowTool).toBeDefined();
    expect(CounterBadgeTool).toBeDefined();
    expect(BlurTool).toBeDefined();
    expect(TextTool).toBeDefined();
    expect(ShapeTool).toBeDefined();
  });
});
