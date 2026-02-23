import { v4 as uuidv4 } from 'uuid';

export type Mood = 'idle' | 'thinking' | 'surprised' | 'happy';

export interface Message {
	id: string;
	role: 'user' | 'ai';
	content: string;
	mood?: Mood;
	timestamp: number;
}

export interface ChatSession {
	id: string;
	title: string;
	groupId: string;
	messages: Message[];
}

export interface TabGroup {
	id: string;
	name: string;
	color: string;
	collapsed: boolean;
	chatIds: string[]; 
}

class AppState {
	// Svelte 5 Runes
	groups = $state<TabGroup[]>([
		{ id: 'g1', name: 'Work', color: '#f59e0b', collapsed: false, chatIds: ['c1'] },
		{ id: 'g2', name: 'Personal', color: '#3b82f6', collapsed: false, chatIds: ['c2'] }
	]);
	
	chats = $state<Record<string, ChatSession>>({
		'c1': { 
			id: 'c1', 
			title: 'Q1 Roadmap', 
			groupId: 'g1', 
			messages: [] 
		},
		'c2': { 
			id: 'c2', 
			title: 'Recipe Ideas', 
			groupId: 'g2', 
			messages: [] 
		}
	});

	activeChatId = $state<string | null>('c1');
	isSidebarOpen = $state(true);
	
	// --- MISSING PROPERTY ADDED HERE ---
	theme = $state<'dark' | 'light'>('dark');
	
	// Drag and Drop State
	draggedChatId = $state<string | null>(null);

	// --- Actions ---
	addChat(groupId: string) {
		const newId = uuidv4();
		this.chats[newId] = {
			id: newId,
			title: 'New Chat',
			groupId,
			messages: []
		};
		const group = this.groups.find(g => g.id === groupId);
		if (group) group.chatIds.push(newId);
		this.activeChatId = newId;
	}

	closeChat(chatId: string) {
		const chat = this.chats[chatId];
		const group = this.groups.find(g => g.id === chat.groupId);
		if (group) {
			group.chatIds = group.chatIds.filter(id => id !== chatId);
		}
		if (this.activeChatId === chatId) {
			this.activeChatId = null;
		}
	}

	renameChat(chatId: string, newTitle: string) {
        const chat = this.chats[chatId];
        if (chat && newTitle.trim() !== "") {
            chat.title = newTitle.trim();
        }
    }
	
	moveTab(chatId: string, targetGroupId: string, targetIndex?: number) {
		const chat = this.chats[chatId];
		const sourceGroup = this.groups.find(g => g.id === chat.groupId);
		const targetGroup = this.groups.find(g => g.id === targetGroupId);

		if (!sourceGroup || !targetGroup) return;

		// 1. Remove from source
		sourceGroup.chatIds = sourceGroup.chatIds.filter(id => id !== chatId);
		
		// 2. Add to target
		if (typeof targetIndex === 'number') {
			targetGroup.chatIds.splice(targetIndex, 0, chatId);
		} else {
			targetGroup.chatIds.push(chatId);
		}

		// 3. Update chat reference
		chat.groupId = targetGroupId;
	}
}

export const appState = new AppState();