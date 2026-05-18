// Advanced TypeScript patterns: generics, enums, decorators, namespaces

enum Status {
  Active = 'active',
  Inactive = 'inactive',
  Pending = 'pending',
}

interface ApiResponse<T> {
  data: T;
  status: number;
  message?: string;
}

type Result<T, E = Error> = { ok: true; data: T } | { ok: false; error: E };

function isOk<T, E>(result: Result<T, E>): result is { ok: true; data: T } {
  return result.ok;
}

class TypedEventEmitter<Events extends Record<string, unknown[]>> {
  private listeners = new Map<keyof Events, Set<Function>>();

  on<K extends keyof Events>(event: K, handler: (...args: Events[K]) => void): this {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(handler);
    return this;
  }

  emit<K extends keyof Events>(event: K, ...args: Events[K]): boolean {
    const handlers = this.listeners.get(event);
    if (!handlers || handlers.size === 0) return false;
    for (const handler of handlers) {
      handler(...args);
    }
    return true;
  }
}

// Conditional types and mapped types
type DeepPartial<T> = {
  [P in keyof T]?: T[P] extends object ? DeepPartial<T[P]> : T[P];
};

function deepMerge<T extends object>(target: T, ...sources: DeepPartial<T>[]): T {
  const result = { ...target };
  for (const source of sources) {
    for (const key in source) {
      const val = source[key as keyof typeof source];
      if (val !== undefined && val !== null) {
        if (typeof val === 'object' && !Array.isArray(val)) {
          (result as any)[key] = deepMerge((result as any)[key] ?? {}, val as any);
        } else {
          (result as any)[key] = val;
        }
      }
    }
  }
  return result;
}

// Async with generics
async function fetchTyped<T>(url: string): Promise<Result<ApiResponse<T>>> {
  try {
    const response = await fetch(url);
    if (!response.ok) {
      return { ok: false, error: new Error(`HTTP ${response.status}`) };
    }
    const data: ApiResponse<T> = await response.json();
    return { ok: true, data };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error : new Error(String(error)),
    };
  }
}

// Discriminated union pattern
type Shape =
  | { kind: 'circle'; radius: number }
  | { kind: 'rectangle'; width: number; height: number }
  | { kind: 'triangle'; base: number; height: number };

function area(shape: Shape): number {
  switch (shape.kind) {
    case 'circle':
      return Math.PI * shape.radius ** 2;
    case 'rectangle':
      return shape.width * shape.height;
    case 'triangle':
      return (shape.base * shape.height) / 2;
  }
}

export { Status, TypedEventEmitter, deepMerge, fetchTyped, area, isOk };
export type { ApiResponse, Result, DeepPartial, Shape };
