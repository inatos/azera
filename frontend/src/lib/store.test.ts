import { describe, test, expect } from 'bun:test';
import { generateSessionId, type Chat, type ChatBranch, type ChatMessage, type Persona, type Tag, type ChatGroup, type GeneratedImage, type ImageModel } from './store.utils';

describe('Store Utilities', () => {
    describe('generateSessionId', () => {
        test('generates a string starting with "session_"', () => {
            const id = generateSessionId();
            expect(id.startsWith('session_')).toBe(true);
        });

        test('generates unique IDs', () => {
            const id1 = generateSessionId();
            const id2 = generateSessionId();
            expect(id1).not.toBe(id2);
        });

        test('includes a timestamp component', () => {
            const before = Date.now();
            const id = generateSessionId();
            const after = Date.now();
            
            // Extract timestamp from ID (format: session_<timestamp>_<random>)
            const parts = id.split('_');
            const timestamp = parseInt(parts[1], 10);
            
            expect(timestamp).toBeGreaterThanOrEqual(before);
            expect(timestamp).toBeLessThanOrEqual(after);
        });

        test('has expected format with three parts', () => {
            const id = generateSessionId();
            const parts = id.split('_');
            expect(parts.length).toBe(3);
            expect(parts[0]).toBe('session');
            expect(!isNaN(parseInt(parts[1], 10))).toBe(true);
            expect(parts[2].length).toBe(9); // Random suffix length
        });
    });

    describe('Type Structures', () => {
        describe('Chat type', () => {
            test('has required properties', () => {
                const chat: Chat = {
                    id: 'chat-1',
                    title: 'Test Chat',
                    createdAt: new Date().toISOString(),
                    branches: [],
                    currentBranchId: 'main',
                };

                expect(chat.id).toBe('chat-1');
                expect(chat.title).toBe('Test Chat');
                expect(chat.branches).toEqual([]);
                expect(chat.currentBranchId).toBe('main');
            });

            test('supports optional properties', () => {
                const chat: Chat = {
                    id: 'chat-2',
                    title: 'Test Chat',
                    createdAt: new Date().toISOString(),
                    branches: [],
                    currentBranchId: 'main',
                    groupId: 'group-1',
                    tags: ['tag-1', 'tag-2'],
                    aiPersonaId: 'azera',
                    userPersonaId: 'protag',
                    modelId: 'llama3.2',
                };

                expect(chat.groupId).toBe('group-1');
                expect(chat.tags).toEqual(['tag-1', 'tag-2']);
                expect(chat.aiPersonaId).toBe('azera');
                expect(chat.userPersonaId).toBe('protag');
            });
        });

        describe('ChatBranch type', () => {
            test('has required properties', () => {
                const branch: ChatBranch = {
                    id: 'branch-1',
                    name: 'Main',
                    parentBranchId: null,
                    forkPointMessageId: null,
                    messages: [],
                    createdAt: new Date().toISOString(),
                };

                expect(branch.id).toBe('branch-1');
                expect(branch.name).toBe('Main');
                expect(branch.parentBranchId).toBeNull();
                expect(branch.messages).toEqual([]);
            });

            test('supports forked branches', () => {
                const branch: ChatBranch = {
                    id: 'branch-2',
                    name: 'Fork 1',
                    parentBranchId: 'branch-1',
                    forkPointMessageId: 'msg-5',
                    messages: [],
                    createdAt: new Date().toISOString(),
                };

                expect(branch.parentBranchId).toBe('branch-1');
                expect(branch.forkPointMessageId).toBe('msg-5');
            });
        });

        describe('ChatMessage type', () => {
            test('has required properties', () => {
                const message: ChatMessage = {
                    id: 'msg-1',
                    role: 'user',
                    content: 'Hello!',
                };

                expect(message.id).toBe('msg-1');
                expect(message.role).toBe('user');
                expect(message.content).toBe('Hello!');
            });

            test('supports all roles', () => {
                const roles: Array<'user' | 'assistant' | 'system'> = ['user', 'assistant', 'system'];
                roles.forEach(role => {
                    const message: ChatMessage = {
                        id: `msg-${role}`,
                        role,
                        content: 'Test',
                    };
                    expect(message.role).toBe(role);
                });
            });

            test('supports optional properties', () => {
                const message: ChatMessage = {
                    id: 'msg-2',
                    role: 'assistant',
                    content: 'Hi there!',
                    timestamp: new Date().toISOString(),
                    userPersona: 'user-1',
                    aiPersona: 'azera',
                    model: 'llama3.2',
                    mood: 'happy',
                    thinking: 'Let me think about this...',
                };

                expect(message.timestamp).toBeDefined();
                expect(message.model).toBe('llama3.2');
                expect(message.mood).toBe('happy');
                expect(message.thinking).toBeDefined();
            });
        });

        describe('Persona type', () => {
            test('supports optional properties', () => {
                const persona: Persona = {
                    id: 'persona-1',
                    name: 'Azera',
                    type: 'ai',
                    description: 'Technical assistant',
                    avatar: '⚙',
                    bubbleColor: '#22d3ee',
                    systemPrompt: 'You are a technical AI.',
                    globalMemoryEnabled: true,
                    currentMood: 'focused',
                    voice: {
                        description: 'Clear and precise',
                        pitch: 0.95,
                        rate: 1.0,
                        volume: 1.0,
                    },
                    metadata: { theme: 'technical' },
                    tags: ['tech', 'coding'],
                    createdAt: new Date().toISOString(),
                    updatedAt: new Date().toISOString(),
                };

                expect(persona.avatar).toBe('⚙');
                expect(persona.voice?.pitch).toBe(0.95);
                expect(persona.tags).toContain('tech');
            });

            test('supports user type', () => {
                const persona: Persona = {
                    id: 'user-1',
                    name: 'Protag',
                    type: 'user',
                    description: 'Standard user profile',
                    metadata: {},
                    createdAt: new Date().toISOString(),
                    updatedAt: new Date().toISOString(),
                };

                expect(persona.type).toBe('user');
            });

            test('has required properties', () => {
                const persona: Persona = {
                    id: 'persona-2',
                    name: 'Areza',
                    type: 'ai',
                    description: 'A mystical AI companion',
                    metadata: {},
                    createdAt: new Date().toISOString(),
                    updatedAt: new Date().toISOString(),
                };

                expect(persona.id).toBe('persona-2');
                expect(persona.name).toBe('Areza');
                expect(persona.type).toBe('ai');
            });

        });

        describe('Tag type', () => {
            test('has required properties', () => {
                const tag: Tag = {
                    id: 'tag-1',
                    name: 'Important',
                    color: '#ef4444',
                };

                expect(tag.id).toBe('tag-1');
                expect(tag.name).toBe('Important');
                expect(tag.color).toBe('#ef4444');
            });

            test('color is a hex value', () => {
                const tag: Tag = {
                    id: 'tag-2',
                    name: 'Test',
                    color: '#3b82f6',
                };

                expect(tag.color).toMatch(/^#[0-9a-fA-F]{6}$/);
            });
        });

        describe('ChatGroup type', () => {
            test('has required properties', () => {
                const group: ChatGroup = {
                    id: 'group-1',
                    name: 'General',
                    color: '#6b7280',
                    collapsed: false,
                    order: 0,
                };

                expect(group.id).toBe('group-1');
                expect(group.name).toBe('General');
                expect(group.collapsed).toBe(false);
                expect(group.order).toBe(0);
            });

            test('supports collapsed state', () => {
                const group: ChatGroup = {
                    id: 'group-2',
                    name: 'Archive',
                    color: '#9ca3af',
                    collapsed: true,
                    order: 1,
                };

                expect(group.collapsed).toBe(true);
            });
        });
        
        describe('GeneratedImage type', () => {
            test('has required properties', () => {
                const image: GeneratedImage = {
                    id: 'img-1',
                    filename: 'sunset_20260101.png',
                    url: '/api/images/sunset_20260101.png',
                    prompt: 'A beautiful sunset over mountains',
                    width: 512,
                    height: 512,
                    createdAt: new Date().toISOString(),
                };

                expect(image.id).toBe('img-1');
                expect(image.filename).toBe('sunset_20260101.png');
                expect(image.prompt).toBe('A beautiful sunset over mountains');
                expect(image.width).toBe(512);
                expect(image.height).toBe(512);
            });

            test('supports optional properties', () => {
                const image: GeneratedImage = {
                    id: 'img-2',
                    filename: 'azera_mystic_forest.png',
                    url: '/api/images/azera_mystic_forest.png',
                    prompt: 'A mystical forest at dusk',
                    negativePrompt: 'blurry, low quality',
                    model: 'stable-diffusion-xl',
                    width: 768,
                    height: 512,
                    steps: 30,
                    cfgScale: 7.5,
                    seed: 12345,
                    personaId: 'azera',
                    personaName: 'Azera',
                    createdAt: new Date().toISOString(),
                };

                expect(image.negativePrompt).toBe('blurry, low quality');
                expect(image.model).toBe('stable-diffusion-xl');
                expect(image.seed).toBe(12345);
                expect(image.personaName).toBe('Azera');
            });
        });

        describe('ImageModel type', () => {
            test('has required properties', () => {
                const model: ImageModel = {
                    name: 'stable-diffusion',
                    displayName: 'Stable Diffusion',
                    installed: true,
                };

                expect(model.name).toBe('stable-diffusion');
                expect(model.displayName).toBe('Stable Diffusion');
                expect(model.installed).toBe(true);
            });

            test('supports optional description', () => {
                const model: ImageModel = {
                    name: 'sdxl',
                    displayName: 'Stable Diffusion XL',
                    description: 'High quality 1024x1024 image generation',
                    installed: true,
                };

                expect(model.description).toBe('High quality 1024x1024 image generation');
            });
        });
    });
});