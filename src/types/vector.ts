/**
 * Supported vector annotation tool kinds mirroring Rust `VectorToolKind`.
 */
export type VectorToolKind =
  | 'arrow'
  | 'rectangle'
  | 'ellipse'
  | 'pencil'
  | 'counter_badge'
  | 'counterBadge'
  | 'text'
  | 'blur'
  | 'highlight';

export const VECTOR_TOOL_KINDS: readonly VectorToolKind[] = [
  'arrow',
  'rectangle',
  'ellipse',
  'pencil',
  'counter_badge',
  'text',
  'blur',
  'highlight',
] as const;

/**
 * An individual vector annotation element positioned along the recording timeline.
 * Mirrors Rust `VectorElement`.
 */
export interface VectorElement<T = Record<string, unknown>> {
  id: string;
  tool: VectorToolKind;
  start_ms: number;
  end_ms: number;
  data?: T;
  // Convenience properties commonly accessed directly on vector elements
  points?: number[];
  stroke?: string;
  strokeWidth?: number;
  fill?: string;
  text?: string;
  fontSize?: number;
  fontFamily?: string;
  count?: number;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
}

/**
 * Serialized representation of `TimedVectorLedger`.
 */
export interface TimedVectorLedger {
  elements: VectorElement[];
  next_id?: number;
}

/**
 * Checks whether a vector element is active at the given millisecond timestamp (inclusive).
 */
export function isVectorElementActiveAt(
  element: VectorElement,
  timestampMs: number
): boolean {
  return timestampMs >= element.start_ms && timestampMs <= element.end_ms;
}

/**
 * Filters a list of vector elements returning those active at `timestampMs`.
 */
export function queryActiveVectorElements(
  elements: VectorElement[],
  timestampMs: number
): VectorElement[] {
  return elements.filter((elem) => isVectorElementActiveAt(elem, timestampMs));
}

/**
 * In-memory client ledger managing timed vector annotations.
 */
export class TimedVectorLedgerManager {
  private elements: VectorElement[] = [];
  private nextIdCounter = 0;

  constructor(initialElements: VectorElement[] = []) {
    this.elements = [...initialElements];
  }

  addElement(element: VectorElement): void {
    this.elements.push(element);
  }

  removeElement(id: string): VectorElement | undefined {
    const index = this.elements.findIndex((e) => e.id === id);
    if (index !== -1) {
      return this.elements.splice(index, 1)[0];
    }
    return undefined;
  }

  clear(): void {
    this.elements = [];
  }

  activeAt(timestampMs: number): VectorElement[] {
    return this.elements.filter((e) => isVectorElementActiveAt(e, timestampMs));
  }

  getElements(): VectorElement[] {
    return [...this.elements];
  }

  setElements(elements: VectorElement[]): void {
    this.elements = [...elements];
  }

  generateId(prefix = 'elem'): string {
    this.nextIdCounter += 1;
    return `${prefix}_${this.nextIdCounter}`;
  }

  toJSON(): TimedVectorLedger {
    return {
      elements: [...this.elements],
      next_id: this.nextIdCounter,
    };
  }
}
