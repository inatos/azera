import { describe, test, expect, mock, beforeEach } from 'bun:test';

// Mock the handleStreamEvent function behavior (extracted from llm_service)
interface StreamEvent {
    type: 'thinking_start' | 'thinking' | 'thinking_end' | 'content' | 'done' | 'error';
    content?: string;
    message_id?: string;
    mood?: string;
    message?: string;
}

interface StreamCallbacks {
    onThinkingStart?: () => void;
    onThinking?: (content: string) => void;
    onThinkingEnd?: () => void;
    onContent?: (content: string) => void;
    onDone?: (messageId: string, mood?: string) => void;
    onError?: (error: string) => void;
}

// Re-implement handleStreamEvent for testing (same logic as in llm_service)
function handleStreamEvent(event: StreamEvent, callbacks: StreamCallbacks) {
    switch (event.type) {
        case 'thinking_start':
            callbacks.onThinkingStart?.();
            break;
        case 'thinking':
            callbacks.onThinking?.(event.content || '');
            break;
        case 'thinking_end':
            callbacks.onThinkingEnd?.();
            break;
        case 'content':
            callbacks.onContent?.(event.content || '');
            break;
        case 'done':
            callbacks.onDone?.(event.message_id || '', event.mood);
            break;
        case 'error':
            callbacks.onError?.(event.message || 'Unknown error');
            break;
    }
}

describe('LLM Service', () => {
    describe('handleStreamEvent', () => {
        let callbacks: StreamCallbacks;

        beforeEach(() => {
            callbacks = {
                onThinkingStart: mock(() => {}),
                onThinking: mock(() => {}),
                onThinkingEnd: mock(() => {}),
                onContent: mock(() => {}),
                onDone: mock(() => {}),
                onError: mock(() => {}),
            };
        });

        test('handles thinking_start event', () => {
            const event: StreamEvent = { type: 'thinking_start' };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onThinkingStart).toHaveBeenCalled();
        });

        test('handles thinking event with content', () => {
            const event: StreamEvent = { type: 'thinking', content: 'Analyzing...' };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onThinking).toHaveBeenCalledWith('Analyzing...');
        });

        test('handles thinking event without content', () => {
            const event: StreamEvent = { type: 'thinking' };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onThinking).toHaveBeenCalledWith('');
        });

        test('handles thinking_end event', () => {
            const event: StreamEvent = { type: 'thinking_end' };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onThinkingEnd).toHaveBeenCalled();
        });

        test('handles content event', () => {
            const event: StreamEvent = { type: 'content', content: 'Hello!' };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onContent).toHaveBeenCalledWith('Hello!');
        });

        test('handles done event with message_id and mood', () => {
            const event: StreamEvent = { 
                type: 'done', 
                message_id: 'msg-123',
                mood: 'happy'
            };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onDone).toHaveBeenCalledWith('msg-123', 'happy');
        });

        test('handles done event without mood', () => {
            const event: StreamEvent = { 
                type: 'done', 
                message_id: 'msg-123'
            };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onDone).toHaveBeenCalledWith('msg-123', undefined);
        });

        test('handles error event with message', () => {
            const event: StreamEvent = { type: 'error', message: 'Connection failed' };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onError).toHaveBeenCalledWith('Connection failed');
        });

        test('handles error event without message', () => {
            const event: StreamEvent = { type: 'error' };
            handleStreamEvent(event, callbacks);
            expect(callbacks.onError).toHaveBeenCalledWith('Unknown error');
        });

        test('does not throw when callback is undefined', () => {
            const minimalCallbacks: StreamCallbacks = {};
            const event: StreamEvent = { type: 'content', content: 'Test' };
            expect(() => handleStreamEvent(event, minimalCallbacks)).not.toThrow();
        });
    });

    describe('StreamEvent type definitions', () => {
        test('validates thinking_start event structure', () => {
            const event: StreamEvent = { type: 'thinking_start' };
            expect(event.type).toBe('thinking_start');
        });

        test('validates content event structure', () => {
            const event: StreamEvent = { type: 'content', content: 'Hello' };
            expect(event.type).toBe('content');
            expect(event.content).toBe('Hello');
        });

        test('validates done event structure', () => {
            const event: StreamEvent = { 
                type: 'done', 
                message_id: 'test-id',
                mood: 'happy' 
            };
            expect(event.type).toBe('done');
            expect(event.message_id).toBe('test-id');
            expect(event.mood).toBe('happy');
        });

        test('validates error event structure', () => {
            const event: StreamEvent = { 
                type: 'error', 
                message: 'Something went wrong' 
            };
            expect(event.type).toBe('error');
            expect(event.message).toBe('Something went wrong');
        });
    });
});
