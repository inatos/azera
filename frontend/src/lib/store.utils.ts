/**
 * Utility functions for the store
 * These are extracted to a separate file so they can be tested without Svelte dependencies
 */

/**
 * Generates a unique session ID
 * Format: session_<timestamp>_<random>
 */
export function generateSessionId(): string {
    return `session_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`;
}

/**
 * Type definitions for the store (extracted for testing)
 */

export type Mood = 'idle' | 'thinking' | 'happy' | 'surprised' | 'content' | 'thoughtful' | 'melancholy' | 'curious' | 'excited' | 'calm' | 'concerned';

export interface Tag {
    id: string;
    name: string;
    color: string;
}

export interface ChatGroup {
    id: string;
    name: string;
    color: string;
    collapsed: boolean;
    order: number;
}

export interface VoiceProfile {
    description: string;
    pitch: number;
    rate: number;
    volume: number;
    voiceName?: string;
    useAiTts?: boolean;
    voiceSampleUrl?: string;
    voiceDescription?: string;
    sampleText?: string;
    personaId?: string;
    mood?: string;
}

export interface Persona {
    id: string;
    name: string;
    type: 'ai' | 'user';
    description: string;
    avatar?: string;
    bubbleColor?: string;
    systemPrompt?: string;
    globalMemoryEnabled?: boolean;
    currentMood?: string;
    voice?: VoiceProfile;
    metadata: Record<string, string>;
    tags?: string[];
    createdAt: string;
    updatedAt: string;
}

export interface ChatMessage {
    id: string;
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp?: string;
    userPersona?: string;
    aiPersona?: string;
    model?: string;
    mood?: string;
    thinking?: string;
}

export interface ChatBranch {
    id: string;
    name: string;
    parentBranchId: string | null;
    forkPointMessageId: string | null;
    messages: ChatMessage[];
    createdAt: string;
}

export interface Chat {
    id: string;
    title: string;
    createdAt: string;
    branches: ChatBranch[];
    currentBranchId: string;
    groupId?: string;
    tags?: string[];
    aiPersonaId?: string;
    userPersonaId?: string;
    modelId?: string;
}

export interface JournalEntry {
    id: string;
    date: string;
    title: string;
    content: string;
    mood?: string;
    personaId?: string;
    personaName?: string;
    tags?: string[];
    createdAt: string;
}

export interface AIModel {
    id: string;
    name: string;
    provider: string;
    description?: string;
}

export interface EditorSettings {
    wordWrap: boolean;
    lineNumbers: boolean;
    fontSize: number;
    tabSize: number;
    lineHighlight: boolean;
    quickSuggestions: boolean;
}

export const DEFAULT_EDITOR_SETTINGS: EditorSettings = {
    wordWrap: true,
    lineNumbers: true,
    fontSize: 13,
    tabSize: 4,
    lineHighlight: true,
    quickSuggestions: false,
};

// Image Generation Types
export interface GeneratedImage {
    id: string;
    filename: string;
    url: string;
    prompt: string;
    negativePrompt?: string;
    model?: string;
    width: number;
    height: number;
    steps?: number;
    cfgScale?: number;
    seed?: number;
    personaId?: string;
    personaName?: string;
    createdAt: string;
}

export interface ImageModel {
    name: string;
    displayName: string;
    description?: string;
    installed: boolean;
}
