/**
 * Ambient type declarations for bun:test
 * Tests run via Bun runtime but svelte-check needs these types.
 */
declare module 'bun:test' {
    export function describe(name: string, fn: () => void): void;
    export function test(name: string, fn: () => void | Promise<void>): void;
    export function expect<T>(value: T): {
        toBe(expected: T): void;
        toEqual(expected: unknown): void;
        toBeTruthy(): void;
        toBeFalsy(): void;
        toBeNull(): void;
        toBeUndefined(): void;
        toBeDefined(): void;
        toBeGreaterThan(expected: number): void;
        toBeGreaterThanOrEqual(expected: number): void;
        toBeLessThan(expected: number): void;
        toBeLessThanOrEqual(expected: number): void;
        toContain(expected: unknown): void;
        toMatch(expected: string | RegExp): void;
        toHaveLength(expected: number): void;
        toThrow(expected?: string | RegExp | Error): void;
        toHaveBeenCalled(): void;
        toHaveBeenCalledTimes(expected: number): void;
        toHaveBeenCalledWith(...args: unknown[]): void;
        not: {
            toBe(expected: T): void;
            toEqual(expected: unknown): void;
            toBeTruthy(): void;
            toBeFalsy(): void;
            toBeNull(): void;
            toBeUndefined(): void;
            toBeDefined(): void;
            toContain(expected: unknown): void;
            toMatch(expected: string | RegExp): void;
            toHaveLength(expected: number): void;
            toThrow(expected?: string | RegExp | Error): void;
            toHaveBeenCalled(): void;
            toHaveBeenCalledTimes(expected: number): void;
            toHaveBeenCalledWith(...args: unknown[]): void;
        };
    };
    export function mock(fn?: (...args: unknown[]) => unknown): {
        (...args: unknown[]): unknown;
        mock: {
            calls: unknown[][];
            results: { type: string; value: unknown }[];
        };
    };
    export function beforeEach(fn: () => void | Promise<void>): void;
    export function afterEach(fn: () => void | Promise<void>): void;
    export function beforeAll(fn: () => void | Promise<void>): void;
    export function afterAll(fn: () => void | Promise<void>): void;
}
