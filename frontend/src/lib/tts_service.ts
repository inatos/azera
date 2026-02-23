import type { VoiceProfile } from './store.svelte';

// Default voice profile
export const defaultVoice: VoiceProfile = {
    description: 'A neutral, clear voice for reading text.',
    pitch: 1.0,
    rate: 1.0,
    volume: 1.0,
};

// TTS state (plain variables - components poll via getters)
let currentUtterance: SpeechSynthesisUtterance | null = null;
let isSpeaking = false;
let isLoading = false;  // True while generating AI TTS audio
let currentMessageId: string | null = null;
let currentAudio: HTMLAudioElement | null = null;  // For AI TTS playback

// API base URL for backend calls
const API_BASE = 'http://localhost:3000';

// Get available voices
export function getAvailableVoices(): SpeechSynthesisVoice[] {
    if (typeof window === 'undefined' || !window.speechSynthesis) {
        return [];
    }
    return window.speechSynthesis.getVoices();
}

// Find the best matching voice based on profile description
function selectVoiceForProfile(profile: VoiceProfile): SpeechSynthesisVoice | null {
    const voices = getAvailableVoices();
    if (voices.length === 0) return null;
    
    // If a specific voice name is requested, try to find it
    if (profile.voiceName) {
        const exact = voices.find(v => v.name === profile.voiceName);
        if (exact) return exact;
    }
    
    // Analyze the description for voice characteristics
    const desc = profile.description.toLowerCase();
    
    // Check for gender hints
    const isFeminine = /ethereal|soft|gentle|melodic|whisper|serene|mystical|female|woman|she|her/.test(desc);
    const isMasculine = /deep|powerful|commanding|male|man|he|his|technical|precise/.test(desc);
    
    // Filter by language (prefer English)
    const englishVoices = voices.filter(v => v.lang.startsWith('en'));
    const candidateVoices = englishVoices.length > 0 ? englishVoices : voices;
    
    // Try to match gender preference
    if (isFeminine) {
        // Look for female voices (common naming patterns)
        const femaleVoice = candidateVoices.find(v => 
            /female|woman|samantha|victoria|karen|moira|fiona|susan|zira|hazel|aria/i.test(v.name)
        );
        if (femaleVoice) return femaleVoice;
    } else if (isMasculine) {
        // Look for male voices
        const maleVoice = candidateVoices.find(v => 
            /male|man|daniel|david|alex|james|thomas|guy|microsoft david/i.test(v.name)
        );
        if (maleVoice) return maleVoice;
    }
    
    // Default to first available English voice, or just the first voice
    return candidateVoices[0] || voices[0] || null;
}

// Speak text with given voice profile
export function speak(text: string, profile: VoiceProfile = defaultVoice, messageId?: string): void {
    // Stop any current speech first
    stop();
    
    // Check if AI TTS is enabled
    if (profile.useAiTts) {
        speakWithAiTts(text, profile, messageId);
        return;
    }
    
    // Use browser TTS
    if (typeof window === 'undefined' || !window.speechSynthesis) {
        console.warn('Speech synthesis not available');
        return;
    }
    
    // Clean the text (remove markdown, code blocks, etc.)
    const cleanText = cleanTextForSpeech(text);
    
    // Apply mood-based modulation
    const { pitch: moodPitch, rate: moodRate } = applyMoodModulation(profile);
    
    // Create utterance
    const utterance = new SpeechSynthesisUtterance(cleanText);
    
    // Apply voice profile settings with mood modulation
    utterance.pitch = moodPitch;
    utterance.rate = moodRate;
    utterance.volume = profile.volume;
    
    // Select voice
    const voice = selectVoiceForProfile(profile);
    if (voice) {
        utterance.voice = voice;
    }
    
    // Set up event handlers
    utterance.onstart = () => {
        isSpeaking = true;
        currentMessageId = messageId || null;
    };
    
    utterance.onend = () => {
        isSpeaking = false;
        currentMessageId = null;
        currentUtterance = null;
    };
    
    utterance.onerror = (event) => {
        console.error('Speech synthesis error:', event);
        isSpeaking = false;
        currentMessageId = null;
        currentUtterance = null;
    };
    
    // Store and start
    currentUtterance = utterance;
    window.speechSynthesis.speak(utterance);
}

// Apply mood-based voice modulation
export function applyMoodModulation(profile: VoiceProfile): { pitch: number; rate: number } {
    let pitch = profile.pitch;
    let rate = profile.rate;
    const mood = profile.mood?.toLowerCase();
    
    if (!mood) return { pitch, rate };
    
    // Modulate pitch and rate based on mood
    switch (mood) {
        case 'happy':
        case 'excited':
            pitch = Math.min(2.0, pitch * 1.15);  // Higher, more energetic
            rate = Math.min(2.0, rate * 1.1);
            break;
        case 'thoughtful':
        case 'thinking':
            pitch = pitch * 0.95;  // Slightly lower, slower
            rate = Math.max(0.5, rate * 0.9);
            break;
        case 'melancholy':
        case 'concerned':
            pitch = pitch * 0.9;  // Lower, slower
            rate = Math.max(0.5, rate * 0.85);
            break;
        case 'surprised':
            pitch = Math.min(2.0, pitch * 1.2);  // Higher pitch
            rate = Math.min(2.0, rate * 1.15);
            break;
        case 'calm':
        case 'content':
            pitch = pitch * 0.98;
            rate = Math.max(0.5, rate * 0.95);
            break;
        case 'curious':
            pitch = Math.min(2.0, pitch * 1.05);
            rate = rate * 1.02;
            break;
    }
    
    return { pitch, rate };
}

// Speak text using AI TTS (Coqui XTTS)
async function speakWithAiTts(text: string, profile: VoiceProfile, messageId?: string): Promise<void> {
    const cleanText = cleanTextForSpeech(text);
    
    console.log('🔊 speakWithAiTts called:', {
        textLen: cleanText.length,
        profile: {
            useAiTts: profile.useAiTts,
            voiceSampleUrl: profile.voiceSampleUrl,
            personaId: profile.personaId,
            mood: profile.mood,
        }
    });
    
    try {
        isLoading = true;  // Show loading indicator
        isSpeaking = true;
        currentMessageId = messageId || null;
        
        const requestBody = {
            text: cleanText,
            model: 'xtts',
            voice_sample_url: profile.voiceSampleUrl,
            voice_description: profile.voiceDescription,
            persona_id: profile.personaId,  // Pass persona ID for backend voice lookup
        };
        
        console.log('🔊 Sending TTS request:', requestBody);
        
        const response = await fetch(`${API_BASE}/api/tts/synthesize`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(requestBody),
        });
        
        if (!response.ok) {
            const error = await response.text();
            console.warn('AI TTS not available, falling back to browser TTS:', error);
            
            // Fall back to browser TTS
            isLoading = false;
            isSpeaking = false;
            speakWithBrowserTts(cleanText, profile, messageId);
            return;
        }
        
        const data = await response.json();
        
        // Apply mood-based modulation
        const { pitch: moodPitch, rate: moodRate } = applyMoodModulation(profile);
        
        // Loading complete - audio is ready
        isLoading = false;
        
        // Play the audio
        const audioData = `data:audio/${data.format};base64,${data.audio_base64}`;
        currentAudio = new Audio(audioData);
        currentAudio.volume = profile.volume;
        // Apply mood-based playback rate (rate 1.0 = normal speed)
        currentAudio.playbackRate = moodRate;
        
        currentAudio.onended = () => {
            isSpeaking = false;
            currentMessageId = null;
            currentAudio = null;
        };
        
        // Wait for audio to be fully loaded before playing
        await new Promise<void>((resolve, reject) => {
            if (!currentAudio) {
                reject(new Error('Audio not initialized'));
                return;
            }
            currentAudio.oncanplaythrough = () => resolve();
            currentAudio.onerror = (e) => reject(e);
            // Also resolve if already ready
            if (currentAudio.readyState >= 4) {
                resolve();
            }
        });
        
        // Small delay to ensure audio buffer is fully ready
        await new Promise(resolve => setTimeout(resolve, 100));
        
        // Set error handler for playback errors
        currentAudio.onerror = (e) => {
            console.error('Audio playback error:', e);
            isSpeaking = false;
            currentMessageId = null;
            currentAudio = null;
        };
        
        await currentAudio.play();
        
    } catch (error) {
        console.warn('AI TTS error, falling back to browser TTS:', error);
        isLoading = false;
        isSpeaking = false;
        
        // Fall back to browser TTS
        speakWithBrowserTts(cleanText, profile, messageId);
    }
}

// Browser TTS fallback function
function speakWithBrowserTts(cleanText: string, profile: VoiceProfile, messageId?: string): void {
    if (typeof window === 'undefined' || !window.speechSynthesis) {
        console.warn('Speech synthesis not available');
        return;
    }
    
    // Apply mood-based modulation
    const { pitch: moodPitch, rate: moodRate } = applyMoodModulation(profile);
    
    const utterance = new SpeechSynthesisUtterance(cleanText);
    utterance.pitch = moodPitch;
    utterance.rate = moodRate;
    utterance.volume = profile.volume;
    
    // Use AI voice description if available, otherwise use browser voice description
    const profileForSelection: VoiceProfile = {
        ...profile,
        description: profile.voiceDescription || profile.description,
    };
    const voice = selectVoiceForProfile(profileForSelection);
    if (voice) {
        utterance.voice = voice;
    }
    
    utterance.onstart = () => {
        isSpeaking = true;
        currentMessageId = messageId || null;
    };
    
    utterance.onend = () => {
        isSpeaking = false;
        currentMessageId = null;
        currentUtterance = null;
    };
    
    utterance.onerror = (event) => {
        console.error('Speech synthesis error:', event);
        isSpeaking = false;
        currentMessageId = null;
        currentUtterance = null;
    };
    
    currentUtterance = utterance;
    window.speechSynthesis.speak(utterance);
}

// Stop speaking
export function stop(): void {
    // Stop browser TTS
    if (typeof window !== 'undefined' && window.speechSynthesis) {
        window.speechSynthesis.cancel();
    }
    
    // Stop AI TTS audio
    if (currentAudio) {
        currentAudio.pause();
        currentAudio.currentTime = 0;
        currentAudio = null;
    }
    
    isSpeaking = false;
    currentMessageId = null;
    currentUtterance = null;
}

// Pause speaking
export function pause(): void {
    if (typeof window === 'undefined' || !window.speechSynthesis) return;
    window.speechSynthesis.pause();
}

// Resume speaking
export function resume(): void {
    if (typeof window === 'undefined' || !window.speechSynthesis) return;
    window.speechSynthesis.resume();
}

// Check if currently speaking
export function getIsSpeaking(): boolean {
    return isSpeaking;
}

// Check if currently loading (generating) TTS audio
export function getIsLoading(): boolean {
    return isLoading;
}

// Get the currently speaking message ID
export function getCurrentMessageId(): string | null {
    return currentMessageId;
}

// Clean text for speech (remove markdown formatting, code, etc.)
export function cleanTextForSpeech(text: string): string {
    let cleaned = text;
    
    // Remove code blocks
    cleaned = cleaned.replace(/```[\s\S]*?```/g, 'code block omitted');
    cleaned = cleaned.replace(/`[^`]+`/g, '');
    
    // Remove markdown links but keep text
    cleaned = cleaned.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
    
    // Remove markdown emphasis but keep text
    cleaned = cleaned.replace(/\*\*([^*]+)\*\*/g, '$1');
    cleaned = cleaned.replace(/\*([^*]+)\*/g, '$1');
    cleaned = cleaned.replace(/__([^_]+)__/g, '$1');
    cleaned = cleaned.replace(/_([^_]+)_/g, '$1');
    
    // Remove headers
    cleaned = cleaned.replace(/^#{1,6}\s+/gm, '');
    
    // Remove blockquotes
    cleaned = cleaned.replace(/^>\s*/gm, '');
    
    // Remove horizontal rules
    cleaned = cleaned.replace(/^[-*_]{3,}$/gm, '');
    
    // Remove bullet points and numbered lists
    cleaned = cleaned.replace(/^[\s]*[-*+]\s+/gm, '');
    cleaned = cleaned.replace(/^[\s]*\d+\.\s+/gm, '');
    
    // Clean up extra whitespace
    cleaned = cleaned.replace(/\n{3,}/g, '\n\n');
    cleaned = cleaned.trim();
    
    return cleaned;
}

// Initialize voices (some browsers load voices async)
export function initVoices(): Promise<SpeechSynthesisVoice[]> {
    return new Promise((resolve) => {
        if (typeof window === 'undefined' || !window.speechSynthesis) {
            resolve([]);
            return;
        }
        
        const voices = window.speechSynthesis.getVoices();
        if (voices.length > 0) {
            resolve(voices);
            return;
        }
        
        // Some browsers load voices asynchronously
        window.speechSynthesis.onvoiceschanged = () => {
            resolve(window.speechSynthesis.getVoices());
        };
        
        // Timeout fallback
        setTimeout(() => {
            resolve(window.speechSynthesis.getVoices());
        }, 1000);
    });
}

// TTS reactive state (for Svelte components to subscribe to)
export const ttsState = {
    get isSpeaking() { return isSpeaking; },
    get isLoading() { return isLoading; },
    get currentMessageId() { return currentMessageId; },
};

