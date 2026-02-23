import { describe, test, expect } from 'bun:test';
import { cleanTextForSpeech, applyMoodModulation, defaultVoice } from './tts_service';
import type { VoiceProfile } from './store.svelte';

describe('TTS Service', () => {
    describe('cleanTextForSpeech', () => {
        test('removes code blocks', () => {
            const text = 'Here is some code:\n```javascript\nconst x = 1;\n```\nEnd of code.';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('Here is some code:\ncode block omitted\nEnd of code.');
        });

        test('removes inline code', () => {
            const text = 'Use the `console.log()` function.';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('Use the  function.');
        });

        test('preserves link text but removes URLs', () => {
            const text = 'Check out [this link](https://example.com) for more info.';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('Check out this link for more info.');
        });

        test('removes bold markdown but keeps text', () => {
            const text = 'This is **bold** and __also bold__.';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('This is bold and also bold.');
        });

        test('removes italic markdown but keeps text', () => {
            const text = 'This is *italic* and _also italic_.';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('This is italic and also italic.');
        });

        test('removes headers', () => {
            const text = '# Header 1\n## Header 2\nSome text';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('Header 1\nHeader 2\nSome text');
        });

        test('removes blockquotes', () => {
            const text = '> This is a quote\nNormal text';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('This is a quote\nNormal text');
        });

        test('removes bullet points', () => {
            const text = '- Item 1\n* Item 2\n+ Item 3';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('Item 1\nItem 2\nItem 3');
        });

        test('removes numbered lists', () => {
            const text = '1. First\n2. Second\n3. Third';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('First\nSecond\nThird');
        });

        test('removes horizontal rules', () => {
            const text = 'Above\n---\nBelow';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('Above\n\nBelow');
        });

        test('collapses excessive newlines', () => {
            const text = 'First paragraph\n\n\n\nSecond paragraph';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('First paragraph\n\nSecond paragraph');
        });

        test('handles empty string', () => {
            const result = cleanTextForSpeech('');
            expect(result).toBe('');
        });

        test('handles plain text without modifications', () => {
            const text = 'Just some plain text without any formatting.';
            const result = cleanTextForSpeech(text);
            expect(result).toBe('Just some plain text without any formatting.');
        });
    });

    describe('applyMoodModulation', () => {
        test('returns base pitch and rate when no mood', () => {
            const profile: VoiceProfile = {
                description: 'Test voice',
                pitch: 1.0,
                rate: 1.0,
                volume: 1.0,
            };
            const result = applyMoodModulation(profile);
            expect(result.pitch).toBe(1.0);
            expect(result.rate).toBe(1.0);
        });

        test('increases pitch and rate for happy mood', () => {
            const profile: VoiceProfile = {
                description: 'Test voice',
                pitch: 1.0,
                rate: 1.0,
                volume: 1.0,
                mood: 'happy',
            };
            const result = applyMoodModulation(profile);
            expect(result.pitch).toBeGreaterThan(1.0);
            expect(result.rate).toBeGreaterThan(1.0);
        });

        test('increases pitch and rate for excited mood', () => {
            const profile: VoiceProfile = {
                description: 'Test voice',
                pitch: 1.0,
                rate: 1.0,
                volume: 1.0,
                mood: 'excited',
            };
            const result = applyMoodModulation(profile);
            expect(result.pitch).toBeGreaterThan(1.0);
            expect(result.rate).toBeGreaterThan(1.0);
        });

        test('decreases pitch and rate for thoughtful mood', () => {
            const profile: VoiceProfile = {
                description: 'Test voice',
                pitch: 1.0,
                rate: 1.0,
                volume: 1.0,
                mood: 'thoughtful',
            };
            const result = applyMoodModulation(profile);
            expect(result.pitch).toBeLessThan(1.0);
            expect(result.rate).toBeLessThan(1.0);
        });

        test('increases pitch for surprised mood', () => {
            const profile: VoiceProfile = {
                description: 'Test voice',
                pitch: 1.0,
                rate: 1.0,
                volume: 1.0,
                mood: 'surprised',
            };
            const result = applyMoodModulation(profile);
            expect(result.pitch).toBeGreaterThan(1.0);
        });

        test('respects maximum pitch limit of 2.0', () => {
            const profile: VoiceProfile = {
                description: 'Test voice',
                pitch: 1.9,
                rate: 1.0,
                volume: 1.0,
                mood: 'happy',
            };
            const result = applyMoodModulation(profile);
            expect(result.pitch).toBeLessThanOrEqual(2.0);
        });

        test('respects minimum rate limit of 0.5', () => {
            const profile: VoiceProfile = {
                description: 'Test voice',
                pitch: 1.0,
                rate: 0.5,
                volume: 1.0,
                mood: 'melancholy',
            };
            const result = applyMoodModulation(profile);
            expect(result.rate).toBeGreaterThanOrEqual(0.5);
        });

        test('handles case-insensitive mood', () => {
            const profile: VoiceProfile = {
                description: 'Test voice',
                pitch: 1.0,
                rate: 1.0,
                volume: 1.0,
                mood: 'HAPPY',
            };
            const result = applyMoodModulation(profile);
            expect(result.pitch).toBeGreaterThan(1.0);
        });
    });

    describe('defaultVoice', () => {
        test('has default pitch of 1.0', () => {
            expect(defaultVoice.pitch).toBe(1.0);
        });

        test('has default rate of 1.0', () => {
            expect(defaultVoice.rate).toBe(1.0);
        });

        test('has default volume of 1.0', () => {
            expect(defaultVoice.volume).toBe(1.0);
        });

        test('has a description', () => {
            expect(defaultVoice.description).toBeDefined();
            expect(defaultVoice.description.length).toBeGreaterThan(0);
        });
    });
});
