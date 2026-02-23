<script lang="ts">
    import { onDestroy } from 'svelte';
    import type * as Monaco from 'monaco-editor';
    import { appState, type Persona, type VoiceProfile } from '$lib/store.svelte';
    import { speak, stop, getAvailableVoices, initVoices, defaultVoice } from '$lib/tts_service';
    import { fetchPersonaTemplate } from '$lib/llm_service';
    import ColorPicker from './ColorPicker.svelte';
    import 'monaco-editor/min/vs/editor/editor.main.css';
    
    // Cache for the persona template
    let cachedTemplate = $state('');
    
    let editorContainer: HTMLDivElement | undefined = $state(undefined);
    let editorInstance: Monaco.editor.IStandaloneCodeEditor | null = null;
    let monaco: typeof Monaco;
    
    let personaName = $state('');
    let personaDescription = $state('');
    let personaContent = $state('');
    let personaType = $state<'ai' | 'user'>('ai');
    let personaAvatar = $state('');
    let personaBubbleColor = $state('#e879f9');
    let personaTags = $state<string[]>([]);
    let personaGlobalMemory = $state(true);  // Enable cross-chat memory
    let isNewPersona = $state(true);
    
    // Voice settings
    let voiceDescription = $state('');
    let voicePitch = $state(1.0);
    let voiceRate = $state(1.0);
    let voiceVolume = $state(1.0);
    let isTestingVoice = $state(false);
    let availableVoices = $state<SpeechSynthesisVoice[]>([]);
    
    // AI TTS settings
    let useAiTts = $state(false);
    let voiceSampleUrl = $state('');
    let aiVoiceDescription = $state('');  // Describes voice characteristics for AI TTS
    let sampleText = $state('');          // Text to read when testing
    let fileInput: HTMLInputElement | undefined = $state(undefined);
    
    // Initialize voices on mount
    $effect(() => {
        if (appState.personaEditorOpen) {
            initVoices().then(voices => {
                availableVoices = voices;
            });
        }
    });
    
    // Preset colors for quick selection
    const presetColors = [
        '#e879f9', // Magenta
        '#ff8c42', // Orange  
        '#4a9eff', // Blue
        '#22d3ee', // Cyan
        '#a78bfa', // Purple
        '#34d399', // Green
        '#f472b6', // Pink
        '#fbbf24', // Yellow
    ];
    
    // Initialize from editing persona
    $effect(() => {
        if (appState.editingPersona) {
            personaName = appState.editingPersona.name;
            personaDescription = appState.editingPersona.description;
            personaType = appState.editingPersona.type;
            personaAvatar = appState.editingPersona.avatar || '';
            personaBubbleColor = appState.editingPersona.bubbleColor || '#e879f9';
            personaTags = appState.editingPersona.tags ? [...appState.editingPersona.tags] : [];
            personaGlobalMemory = appState.editingPersona.globalMemoryEnabled ?? true;
            personaContent = appState.editingPersona.systemPrompt || JSON.stringify(appState.editingPersona.metadata, null, 2);
            // Load voice settings
            if (appState.editingPersona.voice) {
                voiceDescription = appState.editingPersona.voice.description || '';
                voicePitch = appState.editingPersona.voice.pitch ?? 1.0;
                voiceRate = appState.editingPersona.voice.rate ?? 1.0;
                voiceVolume = appState.editingPersona.voice.volume ?? 1.0;
                // Load AI TTS settings
                useAiTts = appState.editingPersona.voice.useAiTts ?? false;
                voiceSampleUrl = appState.editingPersona.voice.voiceSampleUrl || '';
                aiVoiceDescription = appState.editingPersona.voice.voiceDescription || '';
                sampleText = appState.editingPersona.voice.sampleText || '';
            } else {
                voiceDescription = '';
                voicePitch = 1.0;
                voiceRate = 1.0;
                voiceVolume = 1.0;
                useAiTts = false;
                voiceSampleUrl = '';
                aiVoiceDescription = '';
                sampleText = '';
            }
            isNewPersona = false;
        } else {
            personaName = '';
            personaDescription = '';
            personaAvatar = '';
            personaBubbleColor = '#e879f9';
            personaTags = [];
            personaGlobalMemory = true;  // Default to enabled for new personas
            // For new personas, load the template from backend (only when editor is actually open)
            if (appState.personaEditorOpen) {
                if (personaType === 'ai') {
                    if (cachedTemplate) {
                        personaContent = cachedTemplate;
                    } else {
                        personaContent = '# Loading template...';
                        fetchPersonaTemplate().then(content => {
                            cachedTemplate = content;
                            personaContent = content;
                            if (editorInstance) editorInstance.setValue(personaContent);
                        }).catch(() => {
                            personaContent = '# Persona Profile\n\nDefine your persona here.';
                            if (editorInstance) editorInstance.setValue(personaContent);
                        });
                    }
                } else {
                    personaContent = '# User Profile\n\n## Preferences\n\n- Communication style: casual\n- Expertise level: intermediate';
                }
            }
            // Reset voice settings
            voiceDescription = '';
            voicePitch = 1.0;
            voiceRate = 1.0;
            voiceVolume = 1.0;
            useAiTts = false;
            voiceSampleUrl = '';
            aiVoiceDescription = '';
            sampleText = '';
            isNewPersona = true;
        }
        
        if (editorInstance) {
            editorInstance.setValue(personaContent);
        }
    });
    
    // Initialize Monaco editor when modal opens and container is available
    $effect(() => {
        if (!appState.personaEditorOpen || !editorContainer) {
            return;
        }
        
        // Already initialized - just update content
        if (editorInstance) {
            return;
        }
        
        // Async initialization
        (async () => {
            const mod = await import('monaco-editor');
            monaco = mod;
            
            if (typeof window !== 'undefined') {
                const workerCode = `self.onmessage = function(e) { self.postMessage({ id: e.data.id }); };`;
                const blob = new Blob([workerCode], { type: 'application/javascript' });
                const workerUrl = URL.createObjectURL(blob);
                (window as any).MonacoEnvironment = { getWorkerUrl: () => workerUrl };
            }
            
            // Double-check container still exists after async
            if (!editorContainer || !appState.personaEditorOpen) return;
            
            editorInstance = monaco.editor.create(editorContainer, {
                value: personaContent,
                language: 'markdown',
                theme: 'vs-dark',
                minimap: { enabled: false },
                lineNumbers: 'on',
                wordWrap: 'on',
                automaticLayout: true,
                fontFamily: 'Consolas, "Courier New", monospace',
                fontSize: 13,
                lineHeight: 20,
                scrollBeyondLastLine: false,
                padding: { top: 12, bottom: 12 },
                renderLineHighlight: 'line',
                scrollbar: {
                    vertical: 'auto',
                    horizontal: 'hidden',
                    verticalScrollbarSize: 8,
                },
                overviewRulerBorder: false,
                hideCursorInOverviewRuler: true,
            });
            
            editorInstance.onDidChangeModelContent(() => {
                personaContent = editorInstance!.getValue();
            });
        })();
    });
    
    onDestroy(() => {
        if (editorInstance) {
            try {
                editorInstance.dispose();
            } catch (e) {
                // Ignore disposal errors
            }
            editorInstance = null;
        }
    });
    
    function disposeEditor() {
        if (editorInstance) {
            try {
                editorInstance.dispose();
            } catch (e) {
                // Ignore disposal errors
            }
            editorInstance = null;
        }
    }
    
    function handleSave() {
        if (!personaName.trim()) return;
        
        // Dispose editor before state updates to avoid async cancellation errors
        disposeEditor();
        
        // Build voice profile if any voice settings are customized
        const hasVoiceSettings = voiceDescription.trim() || 
            voicePitch !== 1.0 || 
            voiceRate !== 1.0 || 
            voiceVolume !== 1.0 ||
            useAiTts ||
            voiceSampleUrl.trim() ||
            aiVoiceDescription.trim() ||
            sampleText.trim() ||
            (appState.editingPersona?.voice !== undefined);
        
        const voiceProfile: VoiceProfile | undefined = hasVoiceSettings ? {
            description: voiceDescription.trim(),
            pitch: voicePitch,
            rate: voiceRate,
            volume: voiceVolume,
            useAiTts,
            voiceSampleUrl: useAiTts && voiceSampleUrl.trim() ? voiceSampleUrl.trim() : undefined,
            voiceDescription: useAiTts && aiVoiceDescription.trim() ? aiVoiceDescription.trim() : undefined,
            sampleText: useAiTts && sampleText.trim() ? sampleText.trim() : undefined,
        } : undefined;
        
        if (isNewPersona) {
            const newPersona = appState.createPersona(personaType, personaName, personaDescription);
            newPersona.avatar = personaAvatar || undefined;
            newPersona.bubbleColor = personaBubbleColor;
            newPersona.tags = personaTags.length > 0 ? personaTags : undefined;
            newPersona.voice = voiceProfile;
            newPersona.globalMemoryEnabled = personaGlobalMemory;
            if (personaType === 'ai') {
                newPersona.systemPrompt = personaContent;
            } else {
                try {
                    newPersona.metadata = JSON.parse(personaContent);
                } catch {
                    newPersona.metadata = { content: personaContent };
                }
            }
            appState.updatePersona(newPersona);
        } else if (appState.editingPersona) {
            const updated: Persona = {
                ...appState.editingPersona,
                name: personaName,
                description: personaDescription,
                avatar: personaAvatar || undefined,
                bubbleColor: personaBubbleColor,
                tags: personaTags.length > 0 ? personaTags : undefined,
                voice: voiceProfile,
                globalMemoryEnabled: personaGlobalMemory,
                systemPrompt: personaType === 'ai' ? personaContent : undefined,
                metadata: personaType === 'user' ? { content: personaContent } : appState.editingPersona.metadata,
            };
            appState.updatePersona(updated);
        }
        
        appState.closePersonaEditor();
    }
    
    function handleDelete() {
        if (appState.editingPersona && confirm(`Delete "${appState.editingPersona.name}"?`)) {
            disposeEditor();
            stop(); // Stop any voice testing
            appState.deletePersona(appState.editingPersona.id, appState.editingPersona.type);
            appState.closePersonaEditor();
        }
    }
    
    function handleClose() {
        disposeEditor();
        stop(); // Stop any voice testing
        appState.closePersonaEditor();
    }
    
    function togglePersonaTag(tagId: string) {
        if (personaTags.includes(tagId)) {
            personaTags = personaTags.filter(t => t !== tagId);
        } else {
            personaTags = [...personaTags, tagId];
        }
    }
    
    function getTagById(tagId: string) {
        return appState.tags.find(t => t.id === tagId);
    }
    
    function testVoice() {
        if (isTestingVoice) {
            stop();
            isTestingVoice = false;
            return;
        }
        
        // Use sampleText if provided, otherwise generate default test text
        const testText = sampleText.trim()
            ? sampleText.trim().slice(0, 200)
            : voiceDescription.trim() 
                ? `Hello, I am ${personaName || 'your assistant'}. ${voiceDescription.slice(0, 100)}`
                : `Hello, I am ${personaName || 'your assistant'}. This is a sample of my voice.`;
        
        isTestingVoice = true;
        speak(testText, {
            description: useAiTts ? aiVoiceDescription : voiceDescription || '',
            pitch: voicePitch,
            rate: voiceRate,
            volume: voiceVolume,
            useAiTts,
            voiceSampleUrl: useAiTts && voiceSampleUrl.trim() ? voiceSampleUrl.trim() : undefined,
            voiceDescription: useAiTts && aiVoiceDescription.trim() ? aiVoiceDescription.trim() : undefined,
            sampleText: useAiTts && sampleText.trim() ? sampleText.trim() : undefined,
        });
        
        // Reset testing state after speech ends (approximate timing)
        setTimeout(() => {
            isTestingVoice = false;
        }, useAiTts ? 10000 : (testText.length / 10) * 1000 / voiceRate);
    }
    
    function handleFileUpload(event: Event) {
        const input = event.target as HTMLInputElement;
        const file = input.files?.[0];
        if (!file) return;
        
        // Upload the audio file to the server
        uploadVoiceSample(file);
    }
    
    let isUploadingVoice = $state(false);
    let uploadError = $state('');
    
    async function uploadVoiceSample(file: File) {
        isUploadingVoice = true;
        uploadError = '';
        
        try {
            const formData = new FormData();
            formData.append('file', file);
            
            const response = await fetch('http://localhost:3000/api/voice-samples/upload', {
                method: 'POST',
                body: formData
            });
            
            if (!response.ok) {
                const errorText = await response.text();
                throw new Error(errorText || `Upload failed: ${response.status}`);
            }
            
            const result = await response.json();
            voiceSampleUrl = result.url;
            console.log('Voice sample uploaded:', result);
        } catch (e) {
            console.error('Failed to upload voice sample:', e);
            uploadError = e instanceof Error ? e.message : 'Upload failed';
            // Fallback to local blob URL for preview
            voiceSampleUrl = URL.createObjectURL(file);
        } finally {
            isUploadingVoice = false;
        }
    }
</script>

{#if appState.personaEditorOpen}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_interactive_supports_focus -->
    <div class="overlay" onclick={handleClose} role="presentation">
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
        <div class="editor-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
            <header class="modal-header">
                <h2>{isNewPersona ? 'Create' : 'Edit'} Persona</h2>
                <button class="close-btn" onclick={handleClose} aria-label="Close">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M18 6L6 18M6 6l12 12"/>
                    </svg>
                </button>
            </header>
            
            <div class="modal-body">
                <div class="form-row">
                    <div class="form-group">
                        <span class="form-label">Type</span>
                        <div class="type-toggle">
                            <button 
                                class:active={personaType === 'ai'} 
                                onclick={() => personaType = 'ai'}
                                disabled={!isNewPersona}
                            >
                                🤖 AI Persona
                            </button>
                            <button 
                                class:active={personaType === 'user'} 
                                onclick={() => personaType = 'user'}
                                disabled={!isNewPersona}
                            >
                                👤 User Persona
                            </button>
                        </div>
                    </div>
                </div>
                
                <div class="form-row two-col">
                    <div class="form-group">
                        <label for="persona-name">Name</label>
                        <input 
                            id="persona-name"
                            type="text" 
                            bind:value={personaName} 
                            placeholder="Persona name..."
                        />
                    </div>
                    <div class="form-group">
                        <label for="persona-desc">Description</label>
                        <input 
                            id="persona-desc"
                            type="text" 
                            bind:value={personaDescription} 
                            placeholder="Short description..."
                        />
                    </div>
                </div>
                
                <!-- Avatar & Color Section -->
                <div class="form-row two-col">
                    <div class="form-group">
                        <label for="persona-avatar">Avatar (emoji, icon, or image URL)</label>
                        <div class="avatar-input-row">
                            <div class="avatar-preview" style="--preview-color: {personaBubbleColor}">
                                {#if personaAvatar.startsWith('http') || personaAvatar.startsWith('data:')}
                                    <img src={personaAvatar} alt="Avatar" />
                                {:else}
                                    <span>{personaAvatar || '?'}</span>
                                {/if}
                            </div>
                            <input 
                                id="persona-avatar"
                                type="text" 
                                bind:value={personaAvatar} 
                                placeholder="✦ or 🤖 or https://..."
                            />
                        </div>
                    </div>
                    <div class="form-group">
                        <label for="persona-color">Bubble Color</label>
                        <div class="color-picker-row">
                            <ColorPicker bind:value={personaBubbleColor} />
                            <input 
                                type="text" 
                                bind:value={personaBubbleColor} 
                                placeholder="#e879f9"
                                class="color-hex"
                            />
                        </div>
                        <div class="color-presets">
                            {#each presetColors as color}
                                <button 
                                    class="preset-swatch" 
                                    class:active={personaBubbleColor === color}
                                    style="background: {color}"
                                    onclick={() => personaBubbleColor = color}
                                    title={color}
                                ></button>
                            {/each}
                        </div>
                    </div>
                </div>
                
                <!-- Tags Section -->
                {#if appState.tags.length > 0}
                    <div class="form-group">
                        <span class="form-label">Tags</span>
                        <div class="persona-tags-row">
                            {#each appState.tags as tag (tag.id)}
                                <button 
                                    class="tag-chip" 
                                    class:active={personaTags.includes(tag.id)}
                                    style="--tag-color: {tag.color}"
                                    onclick={() => togglePersonaTag(tag.id)}
                                >
                                    {#if personaTags.includes(tag.id)}
                                        <span class="tag-check">✓</span>
                                    {/if}
                                    {tag.name}
                                </button>
                            {/each}
                        </div>
                    </div>
                {/if}
                
                <!-- Global Memory Toggle (AI only) -->
                {#if personaType === 'ai'}
                    <div class="form-group memory-section">
                        <div class="toggle-row">
                            <label class="toggle-label" for="global-memory">
                                <span class="toggle-icon">🧠</span>
                                <span class="toggle-text">
                                    <span class="toggle-title">Global Memory</span>
                                    <span class="toggle-desc">Remember all conversations across chats</span>
                                </span>
                            </label>
                            <button 
                                id="global-memory"
                                type="button"
                                class="toggle-switch" 
                                class:active={personaGlobalMemory}
                                onclick={() => personaGlobalMemory = !personaGlobalMemory}
                                role="switch"
                                aria-checked={personaGlobalMemory}
                                aria-label="Global Memory"
                            >
                                <span class="toggle-thumb"></span>
                            </button>
                        </div>
                        <p class="memory-hint">
                            {personaGlobalMemory 
                                ? '✨ This persona will learn and grow from all conversations, like a real person.' 
                                : '🔒 This persona will only remember conversations within the current chat.'}
                        </p>
                    </div>
                {/if}
                
                <!-- Voice Section -->
                <div class="form-group voice-section">
                        <div class="voice-header">
                            <span class="form-label">🔊 Voice Profile</span>
                            <button 
                                class="test-voice-btn" 
                                class:testing={isTestingVoice}
                                onclick={testVoice}
                                title={isTestingVoice ? 'Stop' : 'Test voice'}
                            >
                                {#if isTestingVoice}
                                    <svg viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="6" width="12" height="12" rx="2"/></svg>
                                    Stop
                                {:else}
                                    <svg viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
                                    Test
                                {/if}
                            </button>
                        </div>
                        
                        <!-- AI TTS Toggle -->
                        <div class="ai-tts-toggle">
                            <label class="toggle-label" for="ai-tts-toggle">
                                <span class="toggle-text">
                                    <span class="toggle-title">🤖 AI Voice Synthesis</span>
                                    <span class="toggle-desc">Use AI-powered voice cloning for natural speech</span>
                                </span>
                            </label>
                            <button 
                                id="ai-tts-toggle"
                                type="button"
                                class="toggle-switch" 
                                class:active={useAiTts}
                                onclick={() => useAiTts = !useAiTts}
                                role="switch"
                                aria-checked={useAiTts}
                                aria-label="AI Voice Synthesis"
                            >
                                <span class="toggle-thumb"></span>
                            </button>
                        </div>
                        
                        {#if useAiTts}
                            <!-- AI TTS Settings -->
                            <div class="ai-tts-settings">
                                <div class="form-group">
                                    <label for="voice-sample">Voice Sample URL</label>
                                    <div class="sample-input-row">
                                        <input 
                                            id="voice-sample"
                                            type="text" 
                                            bind:value={voiceSampleUrl} 
                                            placeholder="https://... or upload audio file"
                                            disabled={isUploadingVoice}
                                        />
                                        <input 
                                            type="file" 
                                            accept="audio/*" 
                                            bind:this={fileInput}
                                            onchange={handleFileUpload}
                                            style="display: none;"
                                        />
                                        <button type="button" class="upload-btn" onclick={() => fileInput?.click()} title="Upload audio file" disabled={isUploadingVoice}>
                                            {#if isUploadingVoice}
                                                <svg class="spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10" stroke-dasharray="30 30"/></svg>
                                            {:else}
                                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17,8 12,3 7,8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
                                            {/if}
                                        </button>
                                    </div>
                                    {#if uploadError}
                                        <p class="field-error">{uploadError}</p>
                                    {/if}
                                    <p class="field-hint">Provide a sample audio file (5-30s) of the target voice to clone</p>
                                </div>
                                
                                <div class="form-group">
                                    <label for="voice-description-ai">Voice Description</label>
                                    <textarea
                                        id="voice-description-ai"
                                        bind:value={aiVoiceDescription}
                                        placeholder="Cool feminine voice, mellow pitch..."
                                        rows="2"
                                    ></textarea>
                                    <p class="field-hint">Describes voice characteristics to augment the sample, or generate a voice if no sample</p>
                                </div>
                                
                                <div class="form-group">
                                    <label for="sample-text">Sample Text</label>
                                    <textarea
                                        id="sample-text"
                                        bind:value={sampleText}
                                        placeholder="Hello, I am your AI assistant. How can I help you today?"
                                        rows="2"
                                    ></textarea>
                                    <p class="field-hint">Text the AI will read when you click Test</p>
                                </div>
                            </div>
                        {:else}
                            <!-- Browser TTS Settings -->
                            <textarea 
                                class="voice-description"
                                bind:value={voiceDescription}
                                placeholder="Describe the voice... e.g., 'An ethereal whisper, soft and melodic, carrying ancient wisdom...'"
                                rows="3"
                            ></textarea>
                            <div class="voice-sliders">
                                <div class="slider-group">
                                    <div class="slider-label">
                                        <span>Pitch</span>
                                        <span class="slider-value">{voicePitch.toFixed(2)}</span>
                                    </div>
                                    <input type="range" min="0.5" max="2" step="0.05" bind:value={voicePitch} aria-label="Voice pitch" />
                                </div>
                                <div class="slider-group">
                                    <div class="slider-label">
                                        <span>Speed</span>
                                        <span class="slider-value">{voiceRate.toFixed(2)}</span>
                                    </div>
                                    <input type="range" min="0.5" max="2" step="0.05" bind:value={voiceRate} aria-label="Voice speed" />
                                </div>
                                <div class="slider-group">
                                    <div class="slider-label">
                                        <span>Volume</span>
                                        <span class="slider-value">{(voiceVolume * 100).toFixed(0)}%</span>
                                    </div>
                                    <input type="range" min="0" max="1" step="0.05" bind:value={voiceVolume} aria-label="Voice volume" />
                                </div>
                            </div>
                        {/if}
                </div>
                
                <div class="form-group editor-group">
                    <span class="form-label">Profile</span>
                    <div class="monaco-container" bind:this={editorContainer}></div>
                </div>
            </div>
            
            <footer class="modal-footer">
                {#if !isNewPersona}
                    <button class="delete-btn" onclick={handleDelete}>Delete</button>
                {/if}
                <div class="spacer"></div>
                <button class="cancel-btn" onclick={handleClose}>Cancel</button>
                <button class="save-btn" onclick={handleSave} disabled={!personaName.trim()}>
                    {isNewPersona ? 'Create' : 'Save'}
                </button>
            </footer>
        </div>
    </div>
{/if}

<style>
    .overlay {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.7);
        backdrop-filter: blur(4px);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }
    
    .editor-modal {
        width: 90%;
        max-width: 800px;
        max-height: 90vh;
        background: linear-gradient(135deg, rgba(28, 30, 31, 0.98), rgba(24, 26, 27, 0.98));
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 1rem;
        display: flex;
        flex-direction: column;
        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.6), 0 0 40px rgba(232, 121, 249, 0.1);
    }
    
    .modal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 1.25rem 1.5rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.2);
    }
    
    .modal-header h2 {
        font-size: 1.25rem;
        font-weight: 600;
        color: rgba(232, 234, 236, 0.95);
        margin: 0;
    }
    
    .close-btn {
        width: 2rem;
        height: 2rem;
        padding: 0.25rem;
        background: transparent;
        border: none;
        color: rgba(232, 121, 249, 0.7);
        cursor: pointer;
        border-radius: 0.375rem;
        transition: all 0.2s;
    }
    
    .close-btn:hover {
        background: rgba(232, 121, 249, 0.15);
        color: rgba(232, 234, 236, 0.95);
    }
    
    .close-btn svg {
        width: 100%;
        height: 100%;
    }
    
    .modal-body {
        flex: 1;
        padding: 1.5rem;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    
    .form-row {
        display: flex;
        gap: 1rem;
    }
    
    .form-row.two-col > .form-group {
        flex: 1;
    }
    
    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    
    .form-group label {
        font-size: 0.75rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: rgba(232, 121, 249, 0.7);
    }
    
    .form-group input {
        padding: 0.6rem 0.8rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.5rem;
        color: rgba(232, 234, 236, 0.95);
        font-size: 0.875rem;
        outline: none;
        transition: all 0.2s;
    }
    
    .form-group input:focus {
        border-color: rgba(232, 121, 249, 0.5);
        box-shadow: 0 0 12px rgba(232, 121, 249, 0.15);
    }
    
    .form-group input::placeholder {
        color: rgba(139, 146, 149, 0.5);
    }
    
    .type-toggle {
        display: flex;
        gap: 0.5rem;
    }
    
    .type-toggle button {
        flex: 1;
        padding: 0.6rem 1rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.5rem;
        color: rgba(232, 121, 249, 0.7);
        font-size: 0.8rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .type-toggle button:hover:not(:disabled) {
        background: rgba(232, 121, 249, 0.15);
    }
    
    .type-toggle button.active {
        background: rgba(232, 121, 249, 0.25);
        border-color: rgba(232, 121, 249, 0.5);
        color: rgba(232, 234, 236, 0.95);
    }
    
    .type-toggle button:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
    
    .editor-group {
        flex: 1;
        min-height: 300px;
    }
    
    .monaco-container {
        flex: 1;
        min-height: 280px;
        border-radius: 0.5rem;
        border: 1px solid rgba(232, 121, 249, 0.2);
        overflow: hidden;
    }
    
    .modal-footer {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 1rem 1.5rem;
        border-top: 1px solid rgba(232, 121, 249, 0.2);
    }
    
    .spacer {
        flex: 1;
    }
    
    .cancel-btn, .save-btn, .delete-btn {
        padding: 0.6rem 1.25rem;
        border-radius: 0.5rem;
        font-size: 0.875rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .cancel-btn {
        background: transparent;
        border: 1px solid rgba(232, 121, 249, 0.25);
        color: rgba(232, 121, 249, 0.8);
    }
    
    .cancel-btn:hover {
        background: rgba(232, 121, 249, 0.1);
        border-color: rgba(232, 121, 249, 0.4);
    }
    
    .save-btn {
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.35), rgba(192, 80, 210, 0.25));
        border: 1px solid rgba(232, 121, 249, 0.4);
        color: rgba(232, 234, 236, 0.95);
    }
    
    .save-btn:hover:not(:disabled) {
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.5), rgba(192, 80, 210, 0.4));
        box-shadow: 0 4px 15px rgba(232, 121, 249, 0.2);
    }
    
    .save-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    
    .delete-btn {
        background: rgba(239, 68, 68, 0.2);
        border: 1px solid rgba(239, 68, 68, 0.4);
        color: rgba(252, 165, 165, 0.9);
    }
    
    .delete-btn:hover {
        background: rgba(239, 68, 68, 0.3);
        border-color: rgba(239, 68, 68, 0.6);
    }
    
    /* Avatar Input Styles */
    .avatar-input-row {
        display: flex;
        gap: 0.75rem;
        align-items: center;
    }
    
    .avatar-preview {
        width: 3rem;
        height: 3rem;
        flex-shrink: 0;
        border-radius: 50%;
        background: linear-gradient(135deg, color-mix(in srgb, var(--preview-color) 30%, transparent), color-mix(in srgb, var(--preview-color) 20%, transparent));
        border: 2px solid var(--preview-color);
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 1.5rem;
        box-shadow: 0 0 15px color-mix(in srgb, var(--preview-color) 40%, transparent);
        overflow: hidden;
    }
    
    .avatar-preview img {
        width: 100%;
        height: 100%;
        object-fit: cover;
    }
    
    .avatar-preview span {
        color: rgba(232, 234, 236, 0.95);
    }
    
    .avatar-input-row input {
        flex: 1;
    }
    
    /* Color Picker Styles */
    .color-picker-row {
        display: flex;
        gap: 0.5rem;
        align-items: center;
    }
    
    .color-hex {
        flex: 1;
        padding: 0.6rem 0.8rem;
        background: rgba(232, 135, 249, 0.08);
        border: 1px solid rgba(232, 135, 249, 0.2);
        border-radius: 0.5rem;
        color: rgba(232, 234, 236, 0.95);
        font-size: 0.875rem;
        font-family: 'Consolas', monospace;
        outline: none;
        transition: all 0.2s;
    }
    
    .color-hex:focus {
        border-color: rgba(232, 135, 249, 0.5);
        box-shadow: 0 0 12px rgba(232, 135, 249, 0.15);
    }
    
    .color-presets {
        display: flex;
        gap: 0.5rem;
        margin-top: 0.5rem;
        flex-wrap: wrap;
    }
    
    .preset-swatch {
        width: 1.75rem;
        height: 1.75rem;
        border-radius: 50%;
        border: 2px solid transparent;
        cursor: pointer;
        transition: all 0.2s;
        box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
    }
    
    .preset-swatch:hover {
        transform: scale(1.15);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    }
    
    .preset-swatch.active {
        border-color: rgba(232, 234, 236, 0.9);
        transform: scale(1.1);
    }
    
    /* Persona Tags */
    .persona-tags-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.5rem;
    }
    
    .tag-chip {
        display: inline-flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.35rem 0.75rem;
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid var(--tag-color, #6b7280);
        border-radius: 9999px;
        color: var(--pe-text);
        font-size: 0.8rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .tag-chip:hover {
        background: rgba(var(--tag-color), 0.15);
        border-color: var(--tag-color);
    }
    
    .tag-chip.active {
        background: var(--tag-color);
        color: white;
        border-color: var(--tag-color);
    }
    
    .tag-check {
        font-size: 0.7rem;
    }
    
    /* Voice Section */
    .voice-section {
        background: rgba(232, 121, 249, 0.05);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.5rem;
        padding: 1rem;
    }
    
    .voice-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        margin-bottom: 0.75rem;
    }
    
    .test-voice-btn {
        display: flex;
        align-items: center;
        gap: 0.375rem;
        padding: 0.375rem 0.75rem;
        background: rgba(232, 121, 249, 0.15);
        border: 1px solid rgba(232, 121, 249, 0.3);
        border-radius: 0.375rem;
        color: #e879f9;
        font-size: 0.8rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .test-voice-btn:hover {
        background: rgba(232, 121, 249, 0.25);
        border-color: rgba(232, 121, 249, 0.5);
    }
    
    .test-voice-btn.testing {
        background: rgba(239, 68, 68, 0.2);
        border-color: rgba(239, 68, 68, 0.4);
        color: #ef4444;
    }
    
    .test-voice-btn svg {
        width: 1rem;
        height: 1rem;
    }
    
    .voice-description {
        width: 100%;
        padding: 0.75rem;
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid var(--pe-border);
        border-radius: 0.375rem;
        color: var(--pe-text);
        font-size: 0.875rem;
        font-family: inherit;
        resize: vertical;
        min-height: 4rem;
    }
    
    .voice-description::placeholder {
        color: var(--pe-muted);
        font-style: italic;
    }
    
    .voice-description:focus {
        outline: none;
        border-color: var(--pe-accent);
    }
    
    .voice-sliders {
        display: grid;
        grid-template-columns: repeat(3, 1fr);
        gap: 1rem;
        margin-top: 0.75rem;
    }
    
    .slider-group {
        display: flex;
        flex-direction: column;
        gap: 0.375rem;
    }
    
    .slider-label {
        display: flex;
        justify-content: space-between;
        font-size: 0.75rem;
        color: var(--pe-muted);
    }
    
    .slider-value {
        color: var(--pe-accent);
        font-family: monospace;
    }
    
    .slider-group input[type="range"] {
        width: 100%;
        height: 4px;
        background: rgba(232, 121, 249, 0.2);
        border-radius: 2px;
        appearance: none;
        cursor: pointer;
    }
    
    .slider-group input[type="range"]::-webkit-slider-thumb {
        appearance: none;
        width: 14px;
        height: 14px;
        background: #e879f9;
        border-radius: 50%;
        cursor: pointer;
        transition: transform 0.15s;
    }
    
    .slider-group input[type="range"]::-webkit-slider-thumb:hover {
        transform: scale(1.2);
    }
    
    .slider-group input[type="range"]::-moz-range-thumb {
        width: 14px;
        height: 14px;
        background: #e879f9;
        border: none;
        border-radius: 50%;
        cursor: pointer;
    }
    
    /* AI TTS Toggle and Settings */
    .ai-tts-toggle {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0.75rem;
        background: rgba(34, 211, 238, 0.08);
        border: 1px solid rgba(34, 211, 238, 0.2);
        border-radius: 0.5rem;
        margin-bottom: 0.75rem;
    }
    
    .ai-tts-settings {
        background: rgba(34, 211, 238, 0.05);
        border: 1px solid rgba(34, 211, 238, 0.15);
        border-radius: 0.5rem;
        padding: 1rem;
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    
    .ai-tts-settings .form-group {
        margin: 0;
    }
    
    .ai-tts-settings label {
        font-size: 0.8rem;
        font-weight: 600;
        color: var(--pe-muted);
        margin-bottom: 0.375rem;
        display: block;
    }
    
    .field-hint {
        margin-top: 0.375rem;
        font-size: 0.7rem;
        color: rgba(34, 211, 238, 0.7);
        line-height: 1.3;
    }
    
    .field-error {
        margin-top: 0.375rem;
        font-size: 0.7rem;
        color: #ef4444;
        line-height: 1.3;
    }
    
    .spin {
        animation: spin 1s linear infinite;
    }
    
    @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
    }
    
    .ai-tts-settings textarea {
        width: 100%;
        padding: 0.75rem;
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid var(--pe-border);
        border-radius: 0.375rem;
        color: var(--pe-text);
        font-size: 0.875rem;
        font-family: inherit;
        resize: vertical;
        min-height: 3rem;
    }
    
    .ai-tts-settings textarea::placeholder {
        color: rgba(120, 120, 130, 0.6);
    }
    
    .ai-tts-settings textarea:focus {
        outline: none;
        border-color: #22d3ee;
    }
    
    .ai-tts-settings input[type="text"] {
        width: 100%;
        padding: 0.625rem 0.75rem;
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid var(--pe-border);
        border-radius: 0.375rem;
        color: var(--pe-text);
        font-size: 0.875rem;
    }
    
    .ai-tts-settings input[type="text"]::placeholder {
        color: rgba(120, 120, 130, 0.6);
    }
    
    .ai-tts-settings input[type="text"]:focus {
        outline: none;
        border-color: #22d3ee;
    }

    /* Sample input row with upload button */
    .sample-input-row {
        display: flex;
        gap: 0.5rem;
    }
    
    .sample-input-row input[type="text"] {
        flex: 1;
    }
    
    .upload-btn {
        width: 2.5rem;
        height: 2.5rem;
        background: rgba(34, 211, 238, 0.15);
        border: 1px solid rgba(34, 211, 238, 0.3);
        border-radius: 0.375rem;
        color: #22d3ee;
        cursor: pointer;
        transition: all 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 0.5rem;
    }
    
    .upload-btn:hover {
        background: rgba(34, 211, 238, 0.25);
        border-color: rgba(34, 211, 238, 0.5);
    }
    
    .upload-btn svg {
        width: 1rem;
        height: 1rem;
    }
    
    /* Global Memory Toggle Styles */
    .memory-section {
        background: rgba(147, 51, 234, 0.08);
        border: 1px solid rgba(147, 51, 234, 0.2);
        border-radius: 0.75rem;
        padding: 1rem;
    }
    
    .toggle-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 1rem;
    }
    
    .toggle-label {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        cursor: pointer;
    }
    
    .toggle-icon {
        font-size: 1.25rem;
    }
    
    .toggle-text {
        display: flex;
        flex-direction: column;
    }
    
    .toggle-title {
        font-weight: 600;
        font-size: 0.875rem;
        color: var(--pe-text);
    }
    
    .toggle-desc {
        font-size: 0.7rem;
        color: var(--pe-muted);
    }
    
    .toggle-switch {
        position: relative;
        width: 48px;
        height: 26px;
        background: rgba(100, 100, 120, 0.4);
        border: none;
        border-radius: 13px;
        cursor: pointer;
        transition: background 0.2s ease;
        flex-shrink: 0;
    }
    
    .toggle-switch.active {
        background: linear-gradient(135deg, #9333ea 0%, #a855f7 100%);
    }
    
    .toggle-thumb {
        position: absolute;
        top: 3px;
        left: 3px;
        width: 20px;
        height: 20px;
        background: white;
        border-radius: 50%;
        transition: transform 0.2s ease;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    }
    
    .toggle-switch.active .toggle-thumb {
        transform: translateX(22px);
    }
    
    .memory-hint {
        margin-top: 0.75rem;
        padding: 0.5rem 0.75rem;
        background: rgba(0, 0, 0, 0.2);
        border-radius: 0.5rem;
        font-size: 0.75rem;
        color: rgba(200, 180, 220, 0.8);
        line-height: 1.4;
    }
</style>
