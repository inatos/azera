// Type definitions
export type Mood = 'idle' | 'thinking' | 'happy' | 'surprised' | 'content' | 'thoughtful' | 'melancholy' | 'curious' | 'excited' | 'calm' | 'concerned';

export interface Tag {
    id: string;
    name: string;
    color: string;  // Hex color
}

export interface ChatGroup {
    id: string;
    name: string;
    color: string;  // Hex color
    collapsed: boolean;
    order: number;
}

export interface VoiceProfile {
    description: string;      // Creative description of the voice
    pitch: number;            // 0.0 to 2.0 (1.0 = normal) - for browser TTS
    rate: number;             // 0.1 to 10.0 (1.0 = normal) - for browser TTS
    volume: number;           // 0.0 to 1.0
    voiceName?: string;       // Browser voice name (optional, auto-selected)
    
    // AI TTS settings
    useAiTts?: boolean;       // Toggle between browser/AI TTS (default: false = browser)
    voiceSampleUrl?: string;  // URL to uploaded voice sample for cloning
    voiceDescription?: string; // Describes voice characteristics (used to augment sample or generate voice)
    sampleText?: string;      // Text to read out when testing the voice
    
    // Runtime context (set when calling speak)
    personaId?: string;       // Persona ID for loading voice config from backend
    mood?: string;            // Message mood for voice modulation
}

export interface Persona {
    id: string;
    name: string;
    type: 'ai' | 'user';
    description: string;
    avatar?: string;          // URL, emoji, or base64 image
    bubbleColor?: string;     // Hex color for chat bubble
    systemPrompt?: string;
    globalMemoryEnabled?: boolean;  // Enable cross-chat memory (default true)
    currentMood?: string;     // Dynamic mood based on last response
    voice?: VoiceProfile;     // Voice settings for TTS
    metadata: Record<string, string>;
    tags?: string[];          // Array of tag IDs
    createdAt: string;
    updatedAt: string;
}

export interface AIModel {
    id: string;
    name: string;
    provider: string;
    description?: string;
}

export interface JournalEntry {
    id: string;
    date: string;
    title: string;
    content: string;
    mood?: string;
    personaId?: string;   // Which persona created this entry
    personaName?: string; // Persona name for display
    tags?: string[];      // Tag IDs for categorization
    createdAt: string;
}

export interface ChatMessage {
    id: string;
    role: 'user' | 'assistant' | 'system';
    content: string;
    timestamp?: string;
    userPersona?: string;
    aiPersona?: string;
    model?: string;
    mood?: string;  // AI's mood when message was sent (idle, thinking, surprised, happy)
    thinking?: string;  // Captured thinking/reasoning content
}

export interface ChatBranch {
    id: string;
    name: string;
    parentBranchId: string | null;  // null = main branch
    forkPointMessageId: string | null;  // Message ID where this branch diverged
    messages: ChatMessage[];
    createdAt: string;
}

export interface Chat {
    id: string;
    title: string;
    createdAt: string;
    branches: ChatBranch[];
    currentBranchId: string;
    groupId?: string;         // Optional group ID
    tags?: string[];          // Array of tag IDs
    aiPersonaId?: string;     // AI persona for this chat
    userPersonaId?: string;   // User persona for this chat
    modelId?: string;         // Model for this chat
}

export interface EditorSettings {
    wordWrap: boolean;
    lineNumbers: boolean;
    fontSize: number;
    tabSize: number;
    lineHighlight: boolean;
    quickSuggestions: boolean;
}

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

const DEFAULT_EDITOR_SETTINGS: EditorSettings = {
    wordWrap: true,
    lineNumbers: true,
    fontSize: 13,
    tabSize: 4,
    lineHighlight: true,
    quickSuggestions: false,
};

export class AppState {
    // Initialization flag to prevent premature reactions
    isInitialized = $state(false);
    
    // Chat session management
    chats = $state<Chat[]>([]);
    currentChatId = $state<string | null>(null);
    
    // Chat Groups and Tags
    chatGroups = $state<ChatGroup[]>([
        { id: 'default', name: 'General', color: '#6b7280', collapsed: false, order: 0 },
    ]);
    tags = $state<Tag[]>([
        { id: 'important', name: 'Important', color: '#ef4444' },
        { id: 'work', name: 'Work', color: '#3b82f6' },
        { id: 'personal', name: 'Personal', color: '#22c55e' },
        { id: 'ideas', name: 'Ideas', color: '#f59e0b' },
    ]);
    
    // Derived: current chat, branch, and messages
    currentChat = $derived(this.chats.find(c => c.id === this.currentChatId) || null);
    currentBranch = $derived(() => {
        if (!this.currentChat) return null;
        return this.currentChat.branches.find(b => b.id === this.currentChat!.currentBranchId) || this.currentChat.branches[0];
    });
    messages = $derived(this.currentBranch()?.messages || []);
    messageCount = $derived(this.messages.length);
    
    // Message editor state
    messageEditorOpen = $state(false);
    editingMessageId = $state<string | null>(null);
    editingMessageContent = $state('');
    editingMessagePersona = $state<string | null>(null);
    
    // UI state
    currentInput = $state('');
    selectedModel = $state('');  // Empty to show placeholder
    isSidebarOpen = $state(true);
    sidebarView = $state<'history' | 'dreams' | 'logs' | 'settings' | 'personas' | 'journal'>('history');
    scrollToBottomSignal = $state(0); // Counter to trigger scroll
    isLoading = $state(false);
    status = $state<'awake' | 'dreaming' | 'thinking'>('awake');
    sessionId = $state(generateSessionId());
    
    // Editor settings
    editorSettings = $state<EditorSettings>({ ...DEFAULT_EDITOR_SETTINGS });
    editorSettingsVersion = $state(0); // Version counter for reactivity
    
    // User preferences (persisted to localStorage)
    showThinking = $state(true);       // Show AI reasoning/thinking blocks
    sendOnEnter = $state(false);       // false = Ctrl+Enter, true = Enter to send
    
    // Thinking state for AI responses
    isThinking = $state(false);
    thinkingContent = $state('');
    thinkingComplete = $state(false);
    
    // Persona Editor state
    personaEditorOpen = $state(false);
    editingPersona = $state<Persona | null>(null);
    
    // AI Models
    availableModels = $state<AIModel[]>([]);
    isLoadingModels = $state(false);
    
    // Image Generation
    imageModels = $state<ImageModel[]>([]);
    selectedImageModel = $state('stable-diffusion');
    generatedImages = $state<GeneratedImage[]>([]);
    
    // Personas
    aiPersonas = $state<Persona[]>([]);
    
    userPersonas = $state<Persona[]>([
        {
            id: 'protag',
            name: 'Protag',
            type: 'user',
            description: 'The protagonist — your voice in the conversation',
            avatar: '⚡',
            bubbleColor: '#22d3ee',
            metadata: {},
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
        },
    ]);
    
    currentAiPersonaId = $state('azera');
    currentUserPersonaId = $state('protag');
    
    // Derived personas
    currentAiPersona = $derived(this.aiPersonas.find(p => p.id === this.currentAiPersonaId) || this.aiPersonas[0]);
    currentUserPersona = $derived(this.userPersonas.find(p => p.id === this.currentUserPersonaId) || this.userPersonas[0]);
    
    // Journal state
    journalEntries = $state<JournalEntry[]>([]);
    
    // Journal viewer state
    journalViewerOpen = $state(false);
    viewingJournalEntry = $state<JournalEntry | null>(null);
    
    // Mental state
    mood = $state(0.5);
    energy = $state(0.7);
    currentMood = $state<Mood>('idle');
    
    // Dreams (hallucinations from when the AI is idle)
    dreams = $state<{ id: string; timestamp: string; title: string; content: string; mood?: string; personaId?: string; personaName?: string; tags?: string[] }[]>([]);
    
    // Dream viewer state
    dreamViewerOpen = $state(false);
    viewingDream = $state<{ id: string; timestamp: string; title: string; content: string; mood?: string; personaId?: string; personaName?: string; tags?: string[] } | null>(null);
    
    // Logs (system and debug information)
    logs = $state<{ id: string; timestamp: string; level: 'info' | 'debug' | 'warn' | 'error'; message: string }[]>([
        { id: '1', timestamp: new Date(Date.now() - 600000).toISOString(), level: 'info', message: 'Database connected successfully' },
        { id: '2', timestamp: new Date(Date.now() - 300000).toISOString(), level: 'info', message: 'Azera ready to converse' },
        { id: '3', timestamp: new Date(Date.now() - 60000).toISOString(), level: 'debug', message: 'Loaded 5 messages from history' },
    ]);
    
    // Mock Data for "Group-tagged prompts"
    prompts = $state([
        { id: 1, tag: 'Coding', title: 'Refactor Rust', code: 'Refactor this Rust code to be idiomatic...' },
        { id: 2, tag: 'Creative', title: 'Cyberpunk Story', code: 'Write a story set in 2077...' },
        { id: 3, tag: 'Analysis', title: 'Analyze Code', code: 'Analyze the complexity of this code...' }
    ]);

    // Derived state
    promptGroups = $derived.by(() => {
        const groups: Record<string, any[]> = {};
        this.prompts.forEach(p => {
            if (!groups[p.tag]) groups[p.tag] = [];
            groups[p.tag].push(p);
        });
        return groups;
    });
    
    constructor() {
        // Load from localStorage if exists
        if (typeof window !== 'undefined') {
            const savedSession = localStorage.getItem('azera_session');
            if (savedSession) {
                this.sessionId = savedSession;
            } else {
                localStorage.setItem('azera_session', this.sessionId);
            }
            
            // Load chats from localStorage
            const savedChats = localStorage.getItem('azera_chats');
            if (savedChats) {
                try {
                    const parsed = JSON.parse(savedChats);
                    // Migrate old chat format to new branch-based format
                    this.chats = parsed.map((chat: any) => {
                        // If chat already has branches, it's in the new format
                        if (chat.branches && Array.isArray(chat.branches)) {
                            return chat;
                        }
                        // Migrate old format: convert messages array to a single main branch
                        const mainBranchId = `branch_main_${chat.id}`;
                        const oldMessages = chat.messages || [];
                        return {
                            id: chat.id,
                            title: chat.title,
                            createdAt: chat.createdAt,
                            branches: [{
                                id: mainBranchId,
                                name: 'Main',
                                parentBranchId: null,
                                forkPointMessageId: null,
                                messages: oldMessages.map((msg: any, idx: number) => ({
                                    id: msg.id || `msg_migrated_${idx}_${Date.now()}`,
                                    role: msg.role,
                                    content: msg.content,
                                    timestamp: msg.timestamp,
                                    userPersona: msg.userPersona,
                                    aiPersona: msg.aiPersona,
                                    model: msg.model,
                                })),
                                createdAt: chat.createdAt,
                            }],
                            currentBranchId: mainBranchId,
                        } as Chat;
                    });
                    // Save migrated data if any migrations occurred
                    this.saveChats();
                    
                    // Load last selected chat ID
                    const savedChatId = localStorage.getItem('azera_current_chat');
                    if (savedChatId && this.chats.find(c => c.id === savedChatId)) {
                        this.currentChatId = savedChatId;
                    } else if (this.chats.length > 0) {
                        this.currentChatId = this.chats[0].id;
                    } else {
                        this.createNewChat();
                    }
                } catch (e) {
                    console.error('Failed to load chats:', e);
                    this.createNewChat();
                }
            } else {
                // Create first chat on initialization
                this.createNewChat();
            }
            
            // Load personas from localStorage
            this.loadPersonas();
            
            // Load groups and tags from localStorage
            this.loadGroupsAndTags();
            
            // Load editor settings from localStorage
            this.loadEditorSettings();
            
            // Load user preferences from localStorage
            this.loadPreferences();
            
            // Fetch installed models from backend
            this.fetchModels();
            
            // Sync dreams, journal, personas, etc. from backend
            this.syncWithBackend();
            
            // Mark as initialized
            this.isInitialized = true;
        }
    }
    
    // Fetch installed models from backend (with retry for startup timing)
    async fetchModels(retries = 3) {
        this.isLoadingModels = true;
        try {
            const response = await fetch('http://localhost:3000/api/models');
            if (response.ok) {
                const data = await response.json();
                this.availableModels = data.models.map((m: any) => ({
                    id: m.name,
                    name: m.name.split(':')[0], // Display name without tag
                    provider: 'Ollama',
                    description: `Local model (${m.name})`,
                }));
                
                // If no models returned and retries remaining, wait and try again
                if (this.availableModels.length === 0 && retries > 0) {
                    setTimeout(() => this.fetchModels(retries - 1), 3000);
                    return;
                }
                
                // If selected model is not in the list, select the first available
                if (this.availableModels.length > 0 && 
                    !this.availableModels.some(m => m.id === this.selectedModel)) {
                    this.selectedModel = this.availableModels[0].id;
                }
            } else if (retries > 0) {
                setTimeout(() => this.fetchModels(retries - 1), 3000);
                return;
            }
        } catch (e) {
            console.error('Failed to fetch models:', e);
            if (retries > 0) {
                setTimeout(() => this.fetchModels(retries - 1), 3000);
                return;
            }
            // Set default model if backend unavailable after all retries
            this.availableModels = [
                { id: 'llama3.2:latest', name: 'llama3.2', provider: 'Ollama', description: 'Default model' },
            ];
        } finally {
            this.isLoadingModels = false;
        }
    }

    // Fetch image generation models
    async fetchImageModels() {
        try {
            const response = await fetch('http://localhost:3000/api/images/models');
            if (response.ok) {
                const data: ImageModel[] = await response.json();
                this.imageModels = data;
                
                // If selected model is not in the list, select the first available
                if (this.imageModels.length > 0 && 
                    !this.imageModels.some(m => m.name === this.selectedImageModel)) {
                    this.selectedImageModel = this.imageModels[0].name;
                }
            }
        } catch (e) {
            console.error('Failed to fetch image models:', e);
            // Set default model if backend unavailable
            this.imageModels = [
                { name: 'placeholder', displayName: 'Placeholder', description: 'No service configured', installed: true },
            ];
        }
    }

    // Fetch generated images
    async fetchImages() {
        try {
            const response = await fetch('http://localhost:3000/api/images');
            if (response.ok) {
                const data = await response.json();
                this.generatedImages = data.items.map((img: any) => ({
                    id: img.id,
                    filename: img.filename,
                    url: img.url,
                    prompt: img.prompt,
                    negativePrompt: img.negative_prompt,
                    model: img.model,
                    width: img.width,
                    height: img.height,
                    steps: img.steps,
                    cfgScale: img.cfg_scale,
                    seed: img.seed,
                    personaId: img.persona_id,
                    personaName: img.persona_name,
                    createdAt: img.created_at,
                }));
            }
        } catch (e) {
            console.error('Failed to fetch images:', e);
        }
    }

    // Chat Management Methods
    createNewChat(title?: string) {
        const chatId = `chat_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        const mainBranchId = `branch_main_${chatId}`;
        
        const newChat: Chat = {
            id: chatId,
            title: title || `Chat ${new Date().toLocaleDateString()}`,
            createdAt: new Date().toISOString(),
            branches: [{
                id: mainBranchId,
                name: 'Main',
                parentBranchId: null,
                forkPointMessageId: null,
                messages: [],
                createdAt: new Date().toISOString(),
            }],
            currentBranchId: mainBranchId,
            aiPersonaId: this.currentAiPersonaId,
            userPersonaId: this.currentUserPersonaId,
            modelId: this.selectedModel,
        };
        
        this.chats.unshift(newChat); // Add to beginning
        this.currentChatId = chatId;
        this.saveChats();
        this.saveCurrentChatId();
        // Scroll to bottom after creating new chat
        this.triggerScrollToBottom();
    }

    loadChat(chatId: string) {
        const chat = this.chats.find(c => c.id === chatId);
        if (chat) {
            this.currentChatId = chatId;
            
            // Restore chat's persona/model selections, or infer from messages
            let aiPersonaIdToSet = chat.aiPersonaId;
            let userPersonaIdToSet = chat.userPersonaId;
            let modelToSet = chat.modelId;
            
            // If no stored values, try to infer from the last messages
            const branch = chat.branches.find(b => b.id === chat.currentBranchId) || chat.branches[0];
            if (branch?.messages?.length > 0) {
                // Find last AI message for aiPersona and model
                const lastAiMsg = [...branch.messages].reverse().find(m => m.role === 'assistant');
                if (lastAiMsg) {
                    if (!aiPersonaIdToSet && lastAiMsg.aiPersona) {
                        // aiPersona is stored as NAME, find the ID
                        const persona = this.aiPersonas.find(p => p.name === lastAiMsg.aiPersona);
                        if (persona) aiPersonaIdToSet = persona.id;
                    }
                    if (!modelToSet && lastAiMsg.model) {
                        modelToSet = lastAiMsg.model;
                    }
                }
                // Find last user message for userPersona
                const lastUserMsg = [...branch.messages].reverse().find(m => m.role === 'user');
                if (lastUserMsg && !userPersonaIdToSet && lastUserMsg.userPersona) {
                    // userPersona is stored as NAME, find the ID
                    const persona = this.userPersonas.find(p => p.name === lastUserMsg.userPersona);
                    if (persona) userPersonaIdToSet = persona.id;
                }
            }
            
            // Apply selections if valid, otherwise keep defaults
            if (aiPersonaIdToSet && this.aiPersonas.some(p => p.id === aiPersonaIdToSet)) {
                this.currentAiPersonaId = aiPersonaIdToSet;
            }
            if (userPersonaIdToSet && this.userPersonas.some(p => p.id === userPersonaIdToSet)) {
                this.currentUserPersonaId = userPersonaIdToSet;
            }
            if (modelToSet && this.availableModels.some(m => m.id === modelToSet)) {
                this.selectedModel = modelToSet;
            }
            
            this.saveCurrentChatId();
            // Scroll to bottom after loading chat
            this.triggerScrollToBottom();
        }
    }

    deleteChat(chatId: string) {
        this.chats = this.chats.filter(c => c.id !== chatId);
        
        // If deleted chat was current, switch to first available or create new
        if (this.currentChatId === chatId) {
            if (this.chats.length > 0) {
                this.currentChatId = this.chats[0].id;
                this.saveCurrentChatId();
            } else {
                this.createNewChat();
            }
        }
        
        this.saveChats();
    }

    renameChat(chatId: string, newTitle: string) {
        const chat = this.chats.find(c => c.id === chatId);
        if (chat) {
            chat.title = newTitle;
            this.saveChats();
        }
    }
    
    // Chat Group Management
    setChatGroup(chatId: string, groupId: string | undefined) {
        const chat = this.chats.find(c => c.id === chatId);
        if (chat) {
            chat.groupId = groupId;
            this.saveChats();
        }
    }
    
    setChatTags(chatId: string, tagIds: string[]) {
        const chat = this.chats.find(c => c.id === chatId);
        if (chat) {
            chat.tags = tagIds;
            this.saveChats();
        }
    }
    
    addChatTag(chatId: string, tagId: string) {
        const chat = this.chats.find(c => c.id === chatId);
        if (chat) {
            if (!chat.tags) chat.tags = [];
            if (!chat.tags.includes(tagId)) {
                chat.tags.push(tagId);
                this.saveChats();
            }
        }
    }
    
    removeChatTag(chatId: string, tagId: string) {
        const chat = this.chats.find(c => c.id === chatId);
        if (chat && chat.tags) {
            chat.tags = chat.tags.filter(t => t !== tagId);
            this.saveChats();
        }
    }
    
    // Group CRUD
    createGroup(name: string, color: string = '#6b7280'): ChatGroup {
        const group: ChatGroup = {
            id: `group_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            name,
            color,
            collapsed: false,
            order: this.chatGroups.length,
        };
        this.chatGroups.push(group);
        this.saveGroups();
        return group;
    }
    
    updateGroup(groupId: string, updates: Partial<Pick<ChatGroup, 'name' | 'color' | 'collapsed' | 'order'>>) {
        const group = this.chatGroups.find(g => g.id === groupId);
        if (group) {
            Object.assign(group, updates);
            this.saveGroups();
        }
    }
    
    deleteGroup(groupId: string) {
        // Move chats from deleted group to ungrouped
        this.chats.forEach(chat => {
            if (chat.groupId === groupId) {
                chat.groupId = undefined;
            }
        });
        this.chatGroups = this.chatGroups.filter(g => g.id !== groupId);
        this.saveGroups();
        this.saveChats();
    }
    
    toggleGroupCollapse(groupId: string) {
        const group = this.chatGroups.find(g => g.id === groupId);
        if (group) {
            group.collapsed = !group.collapsed;
            this.saveGroups();
        }
    }
    
    // Tag CRUD
    createTag(name: string, color: string = '#6b7280'): Tag {
        const tag: Tag = {
            id: `tag_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            name,
            color,
        };
        this.tags.push(tag);
        this.saveTags();
        return tag;
    }
    
    updateTag(tagId: string, updates: Partial<Pick<Tag, 'name' | 'color'>>) {
        const tag = this.tags.find(t => t.id === tagId);
        if (tag) {
            Object.assign(tag, updates);
            this.saveTags();
        }
    }
    
    deleteTag(tagId: string) {
        // Remove tag from all chats
        this.chats.forEach(chat => {
            if (chat.tags) {
                chat.tags = chat.tags.filter(t => t !== tagId);
            }
        });
        // Remove tag from all personas
        this.aiPersonas.forEach(p => {
            if (p.tags) p.tags = p.tags.filter(t => t !== tagId);
        });
        this.userPersonas.forEach(p => {
            if (p.tags) p.tags = p.tags.filter(t => t !== tagId);
        });
        this.tags = this.tags.filter(t => t.id !== tagId);
        this.saveTags();
        this.saveChats();
        this.savePersonas();
    }
    
    private saveGroups() {
        if (typeof window !== 'undefined') {
            localStorage.setItem('azera_chat_groups', JSON.stringify(this.chatGroups));
        }
    }
    
    private saveTags() {
        if (typeof window !== 'undefined') {
            localStorage.setItem('azera_tags', JSON.stringify(this.tags));
        }
    }
    
    private loadGroupsAndTags() {
        if (typeof window !== 'undefined') {
            const savedGroups = localStorage.getItem('azera_chat_groups');
            const savedTags = localStorage.getItem('azera_tags');
            
            if (savedGroups) {
                try {
                    this.chatGroups = JSON.parse(savedGroups);
                } catch (e) {
                    console.error('Failed to load chat groups:', e);
                }
            }
            
            if (savedTags) {
                try {
                    this.tags = JSON.parse(savedTags);
                } catch (e) {
                    console.error('Failed to load tags:', e);
                }
            }
        }
    }
    
    // Editor settings persistence - saves to both localStorage and backend
    async saveEditorSettings() {
        if (typeof window !== 'undefined') {
            // Save to localStorage for immediate access
            localStorage.setItem('azera_editor_settings', JSON.stringify(this.editorSettings));
            
            // Also save to backend for persistence across sessions/devices
            try {
                await fetch('http://localhost:3000/api/settings/editor', {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(this.editorSettings)
                });
            } catch (e) {
                console.error('Failed to save editor settings to backend:', e);
            }
        }
    }
    
    private async loadEditorSettings() {
        if (typeof window !== 'undefined') {
            // First try to load from backend
            try {
                const response = await fetch('http://localhost:3000/api/settings');
                if (response.ok) {
                    const data = await response.json();
                    if (data.editorSettings) {
                        this.editorSettings = { ...DEFAULT_EDITOR_SETTINGS, ...data.editorSettings };
                        // Also update localStorage
                        localStorage.setItem('azera_editor_settings', JSON.stringify(this.editorSettings));
                        return;
                    }
                }
            } catch (e) {
                console.warn('Failed to load settings from backend, using localStorage:', e);
            }
            
            // Fall back to localStorage
            const saved = localStorage.getItem('azera_editor_settings');
            if (saved) {
                try {
                    const parsed = JSON.parse(saved);
                    this.editorSettings = { ...DEFAULT_EDITOR_SETTINGS, ...parsed };
                } catch (e) {
                    console.error('Failed to load editor settings:', e);
                }
            }
        }
    }
    
    updateEditorSetting<K extends keyof EditorSettings>(key: K, value: EditorSettings[K]) {
        console.log('updateEditorSetting called:', key, value);
        this.editorSettings[key] = value;
        this.editorSettingsVersion++;
        this.saveEditorSettings();
        
        // Dispatch custom event for components that need to react
        if (typeof window !== 'undefined') {
            window.dispatchEvent(new CustomEvent('azera:editorSettingsChanged', {
                detail: { ...this.editorSettings }
            }));
        }
    }
    
    // User preferences persistence
    savePreferences() {
        if (typeof window !== 'undefined') {
            localStorage.setItem('azera_preferences', JSON.stringify({
                showThinking: this.showThinking,
                sendOnEnter: this.sendOnEnter,
                selectedModel: this.selectedModel,
            }));
        }
    }
    
    private loadPreferences() {
        if (typeof window !== 'undefined') {
            const saved = localStorage.getItem('azera_preferences');
            if (saved) {
                try {
                    const prefs = JSON.parse(saved);
                    if (prefs.showThinking !== undefined) this.showThinking = prefs.showThinking;
                    if (prefs.sendOnEnter !== undefined) this.sendOnEnter = prefs.sendOnEnter;
                    if (prefs.selectedModel) this.selectedModel = prefs.selectedModel;
                } catch (e) {
                    console.error('Failed to load preferences:', e);
                }
            }
        }
    }

    private saveChats() {
        if (typeof window !== 'undefined') {
            localStorage.setItem('azera_chats', JSON.stringify(this.chats));
        }
    }
    
    private saveCurrentChatId() {
        if (typeof window !== 'undefined' && this.currentChatId) {
            localStorage.setItem('azera_current_chat', this.currentChatId);
        }
    }

    // UI Actions
    triggerScrollToBottom() {
        this.scrollToBottomSignal++;
    }
    
    // Thinking state management
    startThinking() {
        this.isThinking = true;
        this.thinkingContent = '';
        this.thinkingComplete = false;
        this.status = 'thinking';
    }
    
    updateThinking(content: string) {
        this.thinkingContent = content;
    }
    
    appendThinking(content: string) {
        this.thinkingContent += content;
    }
    
    completeThinking() {
        this.isThinking = false;
        this.thinkingComplete = true;
        this.status = 'awake';
    }
    
    clearThinking() {
        this.isThinking = false;
        this.thinkingContent = '';
        this.thinkingComplete = false;
    }

    // Message management (works with current chat's current branch)
    addMessage(role: 'user' | 'assistant' | 'system', content: string) {
        if (!this.currentChat) {
            this.createNewChat();
        }
        
        const branch = this.currentBranch();
        if (!branch) return;
        
        const message: ChatMessage = { 
            id: `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            role, 
            content,
            timestamp: new Date().toISOString(),
            userPersona: role === 'user' ? this.currentUserPersona?.name : undefined,
            aiPersona: role === 'assistant' ? this.currentAiPersona?.name : undefined,
            model: role === 'assistant' ? this.selectedModel : undefined,
            mood: role === 'assistant' ? this.currentMood : undefined,
        };
        
        branch.messages.push(message);
        this.saveChats();
    }

    async sendMessage() {
        if (!this.currentInput.trim()) return;
        if (!this.currentAiPersona || !this.currentUserPersona) return;
        if (!this.currentChat) {
            this.createNewChat();
        }

        const userMessage = this.currentInput;
        const branch = this.currentBranch();
        if (!branch) return;

        // Add user message
        const userMsgId = `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        const userMsg: ChatMessage = { 
            id: userMsgId,
            role: 'user', 
            content: userMessage,
            timestamp: new Date().toISOString(),
            userPersona: this.currentUserPersona?.name,
            model: this.selectedModel,
        };
        branch.messages.push(userMsg);
        
        this.currentInput = '';
        this.isLoading = true;
        this.status = 'thinking';
        this.saveChats();
        this.triggerScrollToBottom();

        // Create placeholder for assistant message
        const assistantMsgId = `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        const assistantMsg: ChatMessage = {
            id: assistantMsgId,
            role: 'assistant',
            content: '',
            timestamp: new Date().toISOString(),
            aiPersona: this.currentAiPersona?.name,
            model: this.selectedModel,
            mood: this.currentMood,
        };
        branch.messages.push(assistantMsg);
        this.saveChats();

        try {
            // Import dynamically to avoid SSR issues
            const { sendMessageStream } = await import('./llm_service');
            
            await sendMessageStream(
                {
                    chat_id: this.currentChat!.id,
                    branch_id: branch.id,
                    message: userMessage,
                    model: this.selectedModel,
                    user_persona_id: this.currentUserPersonaId,
                    ai_persona_id: this.currentAiPersonaId,
                },
                {
                    onThinkingStart: () => {
                        this.startThinking();
                    },
                    onThinking: (content) => {
                        this.appendThinking(content);
                    },
                    onThinkingEnd: () => {
                        this.completeThinking();
                    },
                    onContent: (content) => {
                        // Append to the assistant message
                        const msg = branch.messages.find(m => m.id === assistantMsgId);
                        if (msg) {
                            msg.content += content;
                            // Trigger reactivity
                            this.chats = [...this.chats];
                        }
                        this.triggerScrollToBottom();
                    },
                    onDone: (messageId, mood, moodValue, energy) => {
                        const msg = branch.messages.find(m => m.id === assistantMsgId);
                        if (msg) {
                            if (mood) {
                                msg.mood = mood;
                                this.currentMood = mood as Mood;
                                
                                // Update the current AI persona's mood locally for immediate UI feedback
                                if (this.currentAiPersona) {
                                    this.currentAiPersona.currentMood = mood;
                                    // Also update in the aiPersonas array
                                    const personaIndex = this.aiPersonas.findIndex(p => p.id === this.currentAiPersonaId);
                                    if (personaIndex !== -1) {
                                        this.aiPersonas[personaIndex].currentMood = mood;
                                    }
                                    this.savePersonas();
                                }
                            }
                            // Sync numeric mood value and energy from Dragonfly
                            if (moodValue !== undefined) {
                                this.mood = moodValue;
                            }
                            if (energy !== undefined) {
                                this.energy = energy;
                            }
                            // Save thinking content to the message
                            if (this.thinkingContent) {
                                msg.thinking = this.thinkingContent;
                            }
                        }
                        this.saveChats();
                        this.isLoading = false;
                        this.status = 'awake';
                        this.clearThinking();
                    },
                    onError: (error) => {
                        console.error('Stream error:', error);
                        const msg = branch.messages.find(m => m.id === assistantMsgId);
                        if (msg) {
                            msg.content = `Error: ${error}`;
                        }
                        this.saveChats();
                        this.isLoading = false;
                        this.status = 'awake';
                        this.clearThinking();
                    },
                }
            );
        } catch (error) {
            console.error('Failed to send message:', error);
            const msg = branch.messages.find(m => m.id === assistantMsgId);
            if (msg) {
                msg.content = `Error: ${error instanceof Error ? error.message : 'Unknown error'}`;
            }
            this.saveChats();
            this.isLoading = false;
            this.status = 'awake';
            this.clearThinking();
        }
    }

    async loadHistory() {
        try {
            const { fetchChats } = await import('./llm_service');
            const chats = await fetchChats();
            if (chats && chats.length > 0) {
                // Merge with existing chats, preferring backend data
                const existingIds = new Set(this.chats.map(c => c.id));
                for (const chat of chats) {
                    if (!existingIds.has(chat.id)) {
                        this.chats.push(chat);
                    }
                }
                this.saveChats();
            }
        } catch (error) {
            console.error('Failed to load history:', error);
        }
    }

    async getStatus() {
        try {
            const { fetchStatus } = await import('./llm_service');
            const data = await fetchStatus();
            this.mood = data.mood_value || 0.5;
            this.energy = data.energy || 0.7;
            this.status = data.is_dreaming ? 'dreaming' : 'awake';
            this.currentMood = (data.mood || 'idle') as Mood;
        } catch (error) {
            console.error('Failed to get status:', error);
        }
    }

    async syncWithBackend() {
        // Sync local data with backend
        try {
            const [
                { fetchPersonas },
                { fetchGroups },
                { fetchTags },
                { fetchDreams },
                { fetchJournal }
            ] = await Promise.all([
                import('./llm_service'),
                import('./llm_service'),
                import('./llm_service'),
                import('./llm_service'),
                import('./llm_service'),
            ]);

            // Fetch personas from backend
            const personas = await fetchPersonas();
            if (personas && personas.length > 0) {
                const aiPersonas = personas.filter((p: any) => p.type === 'ai');
                const userPersonas = personas.filter((p: any) => p.type === 'user');
                
                // Helper to transform voice config from snake_case to camelCase
                const transformVoice = (v: any): VoiceProfile | undefined => {
                    if (!v) return undefined;
                    return {
                        description: v.description || '',
                        pitch: v.pitch ?? 1.0,
                        rate: v.rate ?? 1.0,
                        volume: v.volume ?? 1.0,
                        voiceName: v.voice_name,
                        useAiTts: v.use_ai_tts ?? false,
                        voiceSampleUrl: v.voice_sample_url,
                        voiceDescription: v.voice_description,
                        sampleText: v.sample_text,
                    };
                };
                
                if (aiPersonas.length > 0) {
                    // Map backend format to frontend format
                    this.aiPersonas = aiPersonas.map((p: any) => ({
                        id: p.id,
                        name: p.name,
                        type: 'ai' as const,
                        description: p.description,
                        avatar: p.avatar,
                        bubbleColor: p.bubble_color,
                        systemPrompt: p.system_prompt,
                        globalMemoryEnabled: p.global_memory_enabled ?? true,
                        currentMood: p.current_mood,
                        voice: transformVoice(p.voice),
                        metadata: p.metadata || {},
                        tags: p.tags,
                        createdAt: p.created_at,
                        updatedAt: p.updated_at,
                    }));
                    this.savePersonas();
                }
                
                if (userPersonas.length > 0) {
                    this.userPersonas = userPersonas.map((p: any) => ({
                        id: p.id,
                        name: p.name,
                        type: 'user' as const,
                        description: p.description,
                        avatar: p.avatar,
                        bubbleColor: p.bubble_color,
                        systemPrompt: p.system_prompt,
                        globalMemoryEnabled: p.global_memory_enabled ?? true,
                        currentMood: p.current_mood,
                        voice: transformVoice(p.voice),
                        metadata: p.metadata || {},
                        tags: p.tags,
                        createdAt: p.created_at,
                        updatedAt: p.updated_at,
                    }));
                    this.savePersonas();
                }
            }

            // Fetch groups
            const groups = await fetchGroups();
            if (groups && groups.length > 0) {
                this.chatGroups = groups.map((g: any) => ({
                    id: g.id,
                    name: g.name,
                    color: g.color,
                    collapsed: g.collapsed,
                    order: g.order,
                }));
                this.saveGroups();
            }

            // Fetch tags
            const tags = await fetchTags();
            if (tags && tags.length > 0) {
                this.tags = tags.map((t: any) => ({
                    id: t.id,
                    name: t.name,
                    color: t.color,
                }));
                this.saveTags();
            }

            // Fetch dreams
            const dreams = await fetchDreams();
            if (dreams && dreams.length > 0) {
                this.dreams = dreams.map((d: any) => ({
                    id: d.id,
                    timestamp: d.timestamp,
                    title: d.title,
                    content: d.content,
                    mood: d.mood,
                    personaId: d.persona_id,
                    personaName: d.persona_name,
                    tags: d.tags || [],
                }));
            }

            // Fetch journal
            const journal = await fetchJournal();
            if (journal && journal.length > 0) {
                this.journalEntries = journal.map((e: any) => ({
                    id: e.id,
                    date: e.date,
                    title: e.title,
                    content: e.content,
                    mood: e.mood,
                    personaId: e.persona_id,
                    personaName: e.persona_name,
                    tags: e.tags || [],
                    createdAt: e.created_at,
                }));
            }

        } catch (error) {
            console.error('Failed to sync with backend:', error);
        }
    }

    clearMessages() {
        const branch = this.currentBranch();
        if (branch) {
            branch.messages = [];
            this.saveChats();
        }
    }
    
    // Message Editing Methods
    openMessageEditor(messageId: string) {
        const branch = this.currentBranch();
        if (!branch) return;
        
        const message = branch.messages.find(m => m.id === messageId);
        if (!message || message.role !== 'user') return;
        
        this.editingMessageId = messageId;
        this.editingMessageContent = message.content;
        this.editingMessagePersona = message.userPersona || this.currentUserPersonaId;
        this.messageEditorOpen = true;
    }
    
    closeMessageEditor() {
        this.messageEditorOpen = false;
        this.editingMessageId = null;
        this.editingMessageContent = '';
        this.editingMessagePersona = null;
    }
    
    // Commit message edit - creates a new branch with the edited message
    commitMessageEdit(branchName?: string) {
        if (!this.currentChat || !this.editingMessageId) return;
        
        const branch = this.currentBranch();
        if (!branch) return;
        
        // Find the message index
        const msgIndex = branch.messages.findIndex(m => m.id === this.editingMessageId);
        if (msgIndex === -1) return;
        
        // Create new branch
        const newBranchId = `branch_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
        const branchCount = this.currentChat.branches.length;
        
        // Copy messages up to and including the edit point
        const messagesUpToEdit = branch.messages.slice(0, msgIndex).map(m => ({...m}));
        
        // Add edited message
        const editedMessage: ChatMessage = {
            id: `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            role: 'user',
            content: this.editingMessageContent,
            timestamp: new Date().toISOString(),
            userPersona: this.editingMessagePersona || undefined,
        };
        messagesUpToEdit.push(editedMessage);
        
        const newBranch: ChatBranch = {
            id: newBranchId,
            name: branchName || `Path ${branchCount + 1}`,
            parentBranchId: branch.id,
            forkPointMessageId: this.editingMessageId,
            messages: messagesUpToEdit,
            createdAt: new Date().toISOString(),
        };
        
        this.currentChat.branches.push(newBranch);
        this.currentChat.currentBranchId = newBranchId;
        
        this.saveChats();
        this.closeMessageEditor();
        this.triggerScrollToBottom();
    }
    
    // Branch Management
    switchBranch(branchId: string) {
        if (!this.currentChat) return;
        const branch = this.currentChat.branches.find(b => b.id === branchId);
        if (branch) {
            this.currentChat.currentBranchId = branchId;
            this.saveChats();
            this.triggerScrollToBottom();
        }
    }
    
    renameBranch(branchId: string, newName: string) {
        if (!this.currentChat) return;
        const branch = this.currentChat.branches.find(b => b.id === branchId);
        if (branch) {
            branch.name = newName;
            this.saveChats();
        }
    }
    
    deleteBranch(branchId: string) {
        if (!this.currentChat) return;
        
        // Can't delete the main branch
        const branch = this.currentChat.branches.find(b => b.id === branchId);
        if (!branch || branch.parentBranchId === null) return;
        
        // Remove the branch
        this.currentChat.branches = this.currentChat.branches.filter(b => b.id !== branchId);
        
        // If deleted branch was current, switch to main
        if (this.currentChat.currentBranchId === branchId) {
            const mainBranch = this.currentChat.branches.find(b => b.parentBranchId === null);
            if (mainBranch) {
                this.currentChat.currentBranchId = mainBranch.id;
            }
        }
        
        this.saveChats();
    }
    
    // Get branch tree for UI display
    getBranchTree() {
        if (!this.currentChat) return [];
        
        const branches = this.currentChat.branches;
        const mainBranch = branches.find(b => b.parentBranchId === null);
        if (!mainBranch) return [];
        
        // Build tree structure
        const buildTree = (parentId: string | null): any[] => {
            return branches
                .filter(b => b.parentBranchId === parentId)
                .map(b => ({
                    ...b,
                    children: buildTree(b.id),
                    isCurrent: b.id === this.currentChat!.currentBranchId,
                }));
        };
        
        return buildTree(null);
    }
    
    // Persona Management
    openPersonaEditor(persona?: Persona) {
        this.editingPersona = persona || null;
        this.personaEditorOpen = true;
    }
    
    closePersonaEditor() {
        this.editingPersona = null;
        this.personaEditorOpen = false;
    }
    
    createPersona(type: 'ai' | 'user', name: string, description: string = '') {
        const newPersona: Persona = {
            id: `${type}_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            name,
            type,
            description,
            systemPrompt: type === 'ai' ? 'You are a helpful AI assistant.' : undefined,
            metadata: {},
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
        };
        
        if (type === 'ai') {
            this.aiPersonas.push(newPersona);
        } else {
            this.userPersonas.push(newPersona);
        }
        
        this.savePersonas();
        
        // Sync to backend (fire-and-forget)
        import('./llm_service').then(({ createPersona: apiCreate }) => {
            apiCreate({
                name: newPersona.name,
                type: newPersona.type,
                description: newPersona.description,
                avatar: newPersona.avatar,
                bubble_color: newPersona.bubbleColor,
                system_prompt: newPersona.systemPrompt,
                tags: newPersona.tags,
            }).then((result: any) => {
                // Update local ID with backend-generated ID
                if (result?.id) {
                    const arr = type === 'ai' ? this.aiPersonas : this.userPersonas;
                    const idx = arr.findIndex(p => p.id === newPersona.id);
                    if (idx !== -1) {
                        arr[idx].id = result.id;
                        this.savePersonas();
                    }
                }
            }).catch((e: any) => console.warn('Backend persona create failed:', e));
        });
        
        return newPersona;
    }
    
    updatePersona(persona: Persona) {
        persona.updatedAt = new Date().toISOString();
        
        if (persona.type === 'ai') {
            const idx = this.aiPersonas.findIndex(p => p.id === persona.id);
            if (idx !== -1) {
                this.aiPersonas[idx] = persona;
            }
        } else {
            const idx = this.userPersonas.findIndex(p => p.id === persona.id);
            if (idx !== -1) {
                this.userPersonas[idx] = persona;
            }
        }
        
        this.savePersonas();
        
        // Sync to backend (fire-and-forget)
        import('./llm_service').then(({ updatePersona: apiUpdate }) => {
            apiUpdate(persona.id, {
                name: persona.name,
                description: persona.description,
                avatar: persona.avatar,
                bubble_color: persona.bubbleColor,
                system_prompt: persona.systemPrompt,
                global_memory_enabled: persona.globalMemoryEnabled,
                current_mood: persona.currentMood,
                voice: persona.voice ? {
                    description: persona.voice.description,
                    pitch: persona.voice.pitch,
                    rate: persona.voice.rate,
                    volume: persona.voice.volume,
                    voice_name: persona.voice.voiceName,
                    use_ai_tts: persona.voice.useAiTts,
                    voice_sample_url: persona.voice.voiceSampleUrl,
                    voice_description: persona.voice.voiceDescription,
                    sample_text: persona.voice.sampleText,
                } : undefined,
                metadata: persona.metadata,
                tags: persona.tags,
            }).catch((e: any) => console.warn('Backend persona update failed:', e));
        });
    }
    
    deletePersona(personaId: string, type: 'ai' | 'user') {
        if (type === 'ai') {
            this.aiPersonas = this.aiPersonas.filter(p => p.id !== personaId);
            if (this.currentAiPersonaId === personaId && this.aiPersonas.length > 0) {
                this.currentAiPersonaId = this.aiPersonas[0].id;
            }
        } else {
            this.userPersonas = this.userPersonas.filter(p => p.id !== personaId);
            if (this.currentUserPersonaId === personaId && this.userPersonas.length > 0) {
                this.currentUserPersonaId = this.userPersonas[0].id;
            }
        }
        
        this.savePersonas();
        
        // Sync to backend (fire-and-forget)
        import('./llm_service').then(({ deletePersona: apiDelete }) => {
            apiDelete(personaId).catch((e: any) => console.warn('Backend persona delete failed:', e));
        });
    }
    
    // Set current AI persona and sync to current chat
    setCurrentAiPersona(personaId: string) {
        this.currentAiPersonaId = personaId;
        // Sync to current chat
        const chat = this.chats.find(c => c.id === this.currentChatId);
        if (chat) {
            chat.aiPersonaId = personaId;
            this.saveChats();
        }
        this.savePersonas();
    }
    
    // Set current user persona and sync to current chat
    setCurrentUserPersona(personaId: string) {
        this.currentUserPersonaId = personaId;
        // Sync to current chat
        const chat = this.chats.find(c => c.id === this.currentChatId);
        if (chat) {
            chat.userPersonaId = personaId;
            this.saveChats();
        }
        this.savePersonas();
    }
    
    // Set current model and sync to current chat
    setCurrentModel(modelId: string) {
        this.selectedModel = modelId;
        // Sync to current chat
        const chat = this.chats.find(c => c.id === this.currentChatId);
        if (chat) {
            chat.modelId = modelId;
            this.saveChats();
        }
    }
    
    setPersonaTags(personaId: string, type: 'ai' | 'user', tagIds: string[]) {
        const personas = type === 'ai' ? this.aiPersonas : this.userPersonas;
        const persona = personas.find(p => p.id === personaId);
        if (persona) {
            persona.tags = tagIds;
            this.savePersonas();
        }
    }
    
    addPersonaTag(personaId: string, type: 'ai' | 'user', tagId: string) {
        const personas = type === 'ai' ? this.aiPersonas : this.userPersonas;
        const persona = personas.find(p => p.id === personaId);
        if (persona) {
            if (!persona.tags) persona.tags = [];
            if (!persona.tags.includes(tagId)) {
                persona.tags.push(tagId);
                this.savePersonas();
            }
        }
    }
    
    removePersonaTag(personaId: string, type: 'ai' | 'user', tagId: string) {
        const personas = type === 'ai' ? this.aiPersonas : this.userPersonas;
        const persona = personas.find(p => p.id === personaId);
        if (persona && persona.tags) {
            persona.tags = persona.tags.filter(t => t !== tagId);
            this.savePersonas();
        }
    }
    
    private savePersonas() {
        if (typeof window !== 'undefined') {
            localStorage.setItem('azera_ai_personas', JSON.stringify(this.aiPersonas));
            localStorage.setItem('azera_user_personas', JSON.stringify(this.userPersonas));
            localStorage.setItem('azera_current_ai_persona', this.currentAiPersonaId);
            localStorage.setItem('azera_current_user_persona', this.currentUserPersonaId);
        }
    }
    
    private loadPersonas() {
        if (typeof window !== 'undefined') {
            const savedAiPersonas = localStorage.getItem('azera_ai_personas');
            const savedUserPersonas = localStorage.getItem('azera_user_personas');
            const savedCurrentAi = localStorage.getItem('azera_current_ai_persona');
            const savedCurrentUser = localStorage.getItem('azera_current_user_persona');
            
            if (savedAiPersonas) {
                try {
                    this.aiPersonas = JSON.parse(savedAiPersonas);
                } catch (e) {
                    console.error('Failed to load AI personas:', e);
                }
            }
            
            if (savedUserPersonas) {
                try {
                    this.userPersonas = JSON.parse(savedUserPersonas);
                } catch (e) {
                    console.error('Failed to load user personas:', e);
                }
            }
            
            if (savedCurrentAi) this.currentAiPersonaId = savedCurrentAi;
            if (savedCurrentUser) this.currentUserPersonaId = savedCurrentUser;
        }
    }
    
    // Journal Methods
    openJournalEntry(entry: JournalEntry) {
        this.viewingJournalEntry = entry;
        this.journalViewerOpen = true;
    }
    
    closeJournalViewer() {
        this.journalViewerOpen = false;
        this.viewingJournalEntry = null;
    }
    
    // AI would call this to create entries (not user)
    addJournalEntry(title: string, content: string, mood?: string, personaId?: string) {
        const today = new Date();
        const persona = personaId 
            ? this.aiPersonas.find(p => p.id === personaId) 
            : this.currentAiPersona;
        const entry: JournalEntry = {
            id: `entry_${Date.now()}`,
            date: today.toISOString().split('T')[0],
            title,
            content,
            mood,
            personaId: persona?.id,
            personaName: persona?.name,
            tags: [],
            createdAt: today.toISOString(),
        };
        this.journalEntries.unshift(entry);
        this.saveJournal();
    }
    
    // Dream Methods
    openDream(dream: { id: string; timestamp: string; title: string; content: string; mood?: string }) {
        this.viewingDream = dream;
        this.dreamViewerOpen = true;
    }
    
    closeDreamViewer() {
        this.dreamViewerOpen = false;
        this.viewingDream = null;
    }
    
    private saveJournal() {
        if (typeof window !== 'undefined') {
            localStorage.setItem('azera_journal', JSON.stringify(this.journalEntries));
        }
    }
    
    private loadJournal() {
        if (typeof window !== 'undefined') {
            const saved = localStorage.getItem('azera_journal');
            if (saved) {
                try {
                    this.journalEntries = JSON.parse(saved);
                } catch (e) {
                    console.error('Failed to load journal:', e);
                }
            }
        }
    }
}

export function generateSessionId(): string {
    return `session_${Date.now()}_${Math.random().toString(36).substring(2, 11)}`;
}

export const appState = new AppState();
