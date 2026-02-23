/**
 * LLM Service - Handles communication with the Azera backend
 * Supports Server-Sent Events (SSE) for streaming responses
 */

// API base URL - uses localhost for browser access
const API_URL = 'http://localhost:3000';

export interface StreamEvent {
    type: 'thinking_start' | 'thinking' | 'thinking_end' | 'content' | 'done' | 'error';
    content?: string;
    message_id?: string;
    mood?: string;
    mood_value?: number;
    energy?: number;
    message?: string;
}

export interface ChatRequest {
    chat_id: string;
    branch_id: string;
    message: string;
    model: string;
    user_persona_id?: string;
    ai_persona_id?: string;
}

export interface StreamCallbacks {
    onThinkingStart?: () => void;
    onThinking?: (content: string) => void;
    onThinkingEnd?: () => void;
    onContent?: (content: string) => void;
    onDone?: (messageId: string, mood?: string, moodValue?: number, energy?: number) => void;
    onError?: (error: string) => void;
}

/**
 * Send a message and receive a streaming response via SSE
 */
export async function sendMessageStream(
    request: ChatRequest,
    callbacks: StreamCallbacks
): Promise<void> {
    try {
        const response = await fetch(`${API_URL}/api/chat/stream`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'text/event-stream',
            },
            body: JSON.stringify(request),
        });

        if (!response.ok) {
            const errorText = await response.text();
            callbacks.onError?.(`API error: ${response.status} - ${errorText}`);
            return;
        }

        const reader = response.body?.getReader();
        if (!reader) {
            callbacks.onError?.('No response body');
            return;
        }

        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
            const { done, value } = await reader.read();
            
            if (done) break;

            buffer += decoder.decode(value, { stream: true });
            
            // Process complete SSE events
            const lines = buffer.split('\n');
            buffer = lines.pop() || ''; // Keep incomplete line in buffer

            for (const line of lines) {
                if (line.startsWith('data: ')) {
                    const data = line.slice(6).trim();
                    if (data) {
                        try {
                            const event: StreamEvent = JSON.parse(data);
                            handleStreamEvent(event, callbacks);
                        } catch (e) {
                            console.warn('Failed to parse SSE event:', data);
                        }
                    }
                }
            }
        }

        // Process any remaining buffer
        if (buffer.trim()) {
            if (buffer.startsWith('data: ')) {
                const data = buffer.slice(6).trim();
                if (data) {
                    try {
                        const event: StreamEvent = JSON.parse(data);
                        handleStreamEvent(event, callbacks);
                    } catch (e) {
                        console.warn('Failed to parse final SSE event:', data);
                    }
                }
            }
        }
    } catch (error) {
        callbacks.onError?.(error instanceof Error ? error.message : 'Unknown error');
    }
}

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
            callbacks.onDone?.(event.message_id || '', event.mood, event.mood_value, event.energy);
            break;
        case 'error':
            callbacks.onError?.(event.message || 'Unknown error');
            break;
    }
}

/**
 * Fetch available models from Ollama
 */
export async function fetchModels(): Promise<string[]> {
    try {
        const response = await fetch(`${API_URL}/api/models`);
        if (!response.ok) return ['llama3.2', 'mistral', 'codellama'];
        const data = await response.json();
        return data.models || ['llama3.2', 'mistral', 'codellama'];
    } catch (error) {
        console.error('Failed to fetch models:', error);
        return ['llama3.2', 'mistral', 'codellama'];
    }
}

/**
 * Get AI status (mood, energy, dreaming state)
 */
export async function fetchStatus(): Promise<{
    status: string;
    mood: string;
    mood_value: number;
    energy: number;
    is_dreaming: boolean;
}> {
    try {
        const response = await fetch(`${API_URL}/api/status`);
        if (!response.ok) throw new Error('Failed to fetch status');
        return await response.json();
    } catch (error) {
        console.error('Failed to fetch status:', error);
        return {
            status: 'awake',
            mood: 'idle',
            mood_value: 0.5,
            energy: 0.7,
            is_dreaming: false,
        };
    }
}

/**
 * Update AI mood
 */
export async function updateMood(mood: string): Promise<void> {
    try {
        await fetch(`${API_URL}/api/status/mood`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ mood }),
        });
    } catch (error) {
        console.error('Failed to update mood:', error);
    }
}

// ============================================================
// CRUD Operations for Chats
// ============================================================

export async function fetchChats() {
    const response = await fetch(`${API_URL}/api/chats`);
    if (!response.ok) throw new Error('Failed to fetch chats');
    const data = await response.json();
    return data.items || [];
}

export async function createChat(title?: string, groupId?: string) {
    const response = await fetch(`${API_URL}/api/chats`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ title, group_id: groupId }),
    });
    if (!response.ok) throw new Error('Failed to create chat');
    return await response.json();
}

export async function updateChat(id: string, updates: { title?: string; group_id?: string; tags?: string[] }) {
    const response = await fetch(`${API_URL}/api/chats/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
    });
    if (!response.ok) throw new Error('Failed to update chat');
    return await response.json();
}

export async function deleteChat(id: string) {
    const response = await fetch(`${API_URL}/api/chats/${id}`, { method: 'DELETE' });
    if (!response.ok) throw new Error('Failed to delete chat');
    return await response.json();
}

export async function searchChats(query: string): Promise<{ id: string; title: string }[]> {
    const response = await fetch(`${API_URL}/api/chats/search?q=${encodeURIComponent(query)}`);
    if (!response.ok) return [];
    const data = await response.json();
    return data.hits || [];
}

export async function searchDreams(query: string): Promise<{ id: string; title: string }[]> {
    const response = await fetch(`${API_URL}/api/dreams/search?q=${encodeURIComponent(query)}`);
    if (!response.ok) return [];
    const data = await response.json();
    return data.hits || [];
}

export async function searchJournal(query: string): Promise<{ id: string; title: string }[]> {
    const response = await fetch(`${API_URL}/api/journal/search?q=${encodeURIComponent(query)}`);
    if (!response.ok) return [];
    const data = await response.json();
    return data.hits || [];
}

// ============================================================
// CRUD Operations for Personas
// ============================================================

export async function fetchPersonas() {
    const response = await fetch(`${API_URL}/api/personas`);
    if (!response.ok) throw new Error('Failed to fetch personas');
    const data = await response.json();
    return data.items || [];
}

export async function fetchPersonaTemplate(): Promise<string> {
    const response = await fetch(`${API_URL}/api/personas/template`);
    if (!response.ok) throw new Error('Failed to fetch persona template');
    const data = await response.json();
    return data.content || '';
}

export async function createPersona(persona: {
    name: string;
    type: 'ai' | 'user';
    description: string;
    avatar?: string;
    bubble_color?: string;
    system_prompt?: string;
    tags?: string[];
}) {
    const response = await fetch(`${API_URL}/api/personas`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(persona),
    });
    if (!response.ok) throw new Error('Failed to create persona');
    return await response.json();
}

export async function updatePersona(id: string, updates: Partial<{
    name: string;
    description: string;
    avatar: string;
    bubble_color: string;
    system_prompt: string;
    global_memory_enabled: boolean;
    current_mood: string;
    voice: {
        description: string;
        pitch: number;
        rate: number;
        volume: number;
        voice_name?: string;
        use_ai_tts?: boolean;
        voice_sample_url?: string;
        voice_description?: string;
        sample_text?: string;
    };
    metadata: Record<string, string>;
    tags: string[];
}>) {
    const response = await fetch(`${API_URL}/api/personas/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
    });
    if (!response.ok) throw new Error('Failed to update persona');
    return await response.json();
}

export async function deletePersona(id: string) {
    const response = await fetch(`${API_URL}/api/personas/${id}`, { method: 'DELETE' });
    if (!response.ok) throw new Error('Failed to delete persona');
    return await response.json();
}

// ============================================================
// CRUD Operations for Groups
// ============================================================

export async function fetchGroups() {
    const response = await fetch(`${API_URL}/api/groups`);
    if (!response.ok) throw new Error('Failed to fetch groups');
    const data = await response.json();
    return data.items || [];
}

export async function createGroup(name: string, color: string = '#6b7280') {
    const response = await fetch(`${API_URL}/api/groups`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, color }),
    });
    if (!response.ok) throw new Error('Failed to create group');
    return await response.json();
}

export async function updateGroup(id: string, updates: { name?: string; color?: string; collapsed?: boolean; order?: number }) {
    const response = await fetch(`${API_URL}/api/groups/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
    });
    if (!response.ok) throw new Error('Failed to update group');
    return await response.json();
}

export async function deleteGroup(id: string) {
    const response = await fetch(`${API_URL}/api/groups/${id}`, { method: 'DELETE' });
    if (!response.ok) throw new Error('Failed to delete group');
    return await response.json();
}

// ============================================================
// CRUD Operations for Tags
// ============================================================

export async function fetchTags() {
    const response = await fetch(`${API_URL}/api/tags`);
    if (!response.ok) throw new Error('Failed to fetch tags');
    const data = await response.json();
    return data.items || [];
}

export async function createTag(name: string, color: string = '#6b7280') {
    const response = await fetch(`${API_URL}/api/tags`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, color }),
    });
    if (!response.ok) throw new Error('Failed to create tag');
    return await response.json();
}

export async function updateTag(id: string, updates: { name?: string; color?: string }) {
    const response = await fetch(`${API_URL}/api/tags/${id}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(updates),
    });
    if (!response.ok) throw new Error('Failed to update tag');
    return await response.json();
}

export async function deleteTag(id: string) {
    const response = await fetch(`${API_URL}/api/tags/${id}`, { method: 'DELETE' });
    if (!response.ok) throw new Error('Failed to delete tag');
    return await response.json();
}

// ============================================================
// Dreams and Journal
// ============================================================

export async function fetchDreams() {
    const response = await fetch(`${API_URL}/api/dreams`);
    if (!response.ok) throw new Error('Failed to fetch dreams');
    const data = await response.json();
    return data.items || [];
}

export async function fetchJournal() {
    const response = await fetch(`${API_URL}/api/journal`);
    if (!response.ok) throw new Error('Failed to fetch journal');
    const data = await response.json();
    return data.items || [];
}

export async function fetchLogs() {
    const response = await fetch(`${API_URL}/api/logs`);
    if (!response.ok) throw new Error('Failed to fetch logs');
    const data = await response.json();
    return data.items || [];
}

// ============================================================
// Health Check
// ============================================================

export async function checkHealth(): Promise<boolean> {
    try {
        const response = await fetch(`${API_URL}/health`);
        return response.ok;
    } catch {
        return false;
    }
}