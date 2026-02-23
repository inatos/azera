<script lang="ts">
    import { appState } from '$lib/store.svelte';
    import { page } from '$app/stores';
    import ColorPicker from './ColorPicker.svelte';
    
    type TabType = 'history' | 'dreams' | 'logs' | 'settings' | 'personas' | 'journal';
    
    // Check if we're on the canvas page
    let isCanvasPage = $derived($page.url.pathname === '/canvas');
    
    // Group & Tag management state
    let showGroupManager = $state(false);
    let showTagManager = $state(false);
    let editingGroupId = $state<string | null>(null);
    let editingTagId = $state<string | null>(null);
    let newGroupName = $state('');
    let newGroupColor = $state('#6b7280');
    let newTagName = $state('');
    let newTagColor = $state('#6b7280');
    let editGroupName = $state('');
    let editGroupColor = $state('#6b7280');
    let editTagName = $state('');
    let editTagColor = $state('#6b7280');
    let chatContextMenuId = $state<string | null>(null);
    let contextMenuPosition = $state({ x: 0, y: 0 });
    
    // Filter state
    let selectedTagFilter = $state<string | null>(null);
    let selectedPersonaFilter = $state<string | null>(null);
    
    // Search state
    let searchQuery = $state('');
    let searchResultIds = $state<Set<string> | null>(null);
    let searchDebounceTimer = $state<ReturnType<typeof setTimeout> | null>(null);
    let isSearching = $state(false);
    
    // Dreams search state
    let dreamSearchQuery = $state('');
    let dreamSearchResultIds = $state<Set<string> | null>(null);
    let dreamSearchTimer = $state<ReturnType<typeof setTimeout> | null>(null);
    let isDreamSearching = $state(false);
    
    // Journal search state
    let journalSearchQuery = $state('');
    let journalSearchResultIds = $state<Set<string> | null>(null);
    let journalSearchTimer = $state<ReturnType<typeof setTimeout> | null>(null);
    let isJournalSearching = $state(false);
    
    async function handleSearchInput(value: string) {
        searchQuery = value;
        
        // Clear previous timer
        if (searchDebounceTimer) clearTimeout(searchDebounceTimer);
        
        if (!value.trim()) {
            // Empty query — show all chats
            searchResultIds = null;
            isSearching = false;
            return;
        }
        
        isSearching = true;
        searchDebounceTimer = setTimeout(async () => {
            try {
                const { searchChats } = await import('$lib/llm_service');
                const hits = await searchChats(value.trim());
                searchResultIds = new Set(hits.map(h => h.id));
            } catch (e) {
                console.error('Search failed:', e);
                searchResultIds = null;
            } finally {
                isSearching = false;
            }
        }, 250);
    }
    
    async function handleDreamSearchInput(value: string) {
        dreamSearchQuery = value;
        if (dreamSearchTimer) clearTimeout(dreamSearchTimer);
        
        if (!value.trim()) {
            dreamSearchResultIds = null;
            isDreamSearching = false;
            return;
        }
        
        isDreamSearching = true;
        dreamSearchTimer = setTimeout(async () => {
            try {
                const { searchDreams } = await import('$lib/llm_service');
                const hits = await searchDreams(value.trim());
                dreamSearchResultIds = new Set(hits.map(h => h.id));
            } catch (e) {
                console.error('Dream search failed:', e);
                dreamSearchResultIds = null;
            } finally {
                isDreamSearching = false;
            }
        }, 250);
    }
    
    async function handleJournalSearchInput(value: string) {
        journalSearchQuery = value;
        if (journalSearchTimer) clearTimeout(journalSearchTimer);
        
        if (!value.trim()) {
            journalSearchResultIds = null;
            isJournalSearching = false;
            return;
        }
        
        isJournalSearching = true;
        journalSearchTimer = setTimeout(async () => {
            try {
                const { searchJournal } = await import('$lib/llm_service');
                const hits = await searchJournal(value.trim());
                journalSearchResultIds = new Set(hits.map(h => h.id));
            } catch (e) {
                console.error('Journal search failed:', e);
                journalSearchResultIds = null;
            } finally {
                isJournalSearching = false;
            }
        }, 250);
    }
    
    // Get all personas (AI) for filtering
    let allPersonas = $derived([...appState.aiPersonas]);
    
    // Group chats computed
    let groupedChats = $derived(() => {
        const groups: Record<string, typeof appState.chats> = {};
        const ungrouped: typeof appState.chats = [];
        
        // Filter by search results if active
        let chatsToGroup = appState.chats;
        if (searchResultIds !== null) {
            chatsToGroup = chatsToGroup.filter(c => searchResultIds!.has(c.id));
        }
        
        // Filter by tag if selected
        if (selectedTagFilter) {
            chatsToGroup = chatsToGroup.filter(c => c.tags?.includes(selectedTagFilter!));
        }
        
        for (const chat of chatsToGroup) {
            if (chat.groupId) {
                if (!groups[chat.groupId]) groups[chat.groupId] = [];
                groups[chat.groupId].push(chat);
            } else {
                ungrouped.push(chat);
            }
        }
        return { groups, ungrouped };
    });
    
    // Group dreams by persona
    let groupedDreams = $derived(() => {
        const byPersona: Record<string, typeof appState.dreams> = {};
        const ungrouped: typeof appState.dreams = [];
        
        // Filter by search results if active
        let dreamsToGroup = appState.dreams;
        if (dreamSearchResultIds !== null) {
            dreamsToGroup = dreamsToGroup.filter(d => dreamSearchResultIds!.has(d.id));
        }
        // Filter by tag if selected
        if (selectedTagFilter) {
            dreamsToGroup = dreamsToGroup.filter(d => d.tags?.includes(selectedTagFilter!));
        }
        if (selectedPersonaFilter) {
            dreamsToGroup = dreamsToGroup.filter(d => d.personaId === selectedPersonaFilter);
        }
        
        for (const dream of dreamsToGroup) {
            if (dream.personaId) {
                if (!byPersona[dream.personaId]) byPersona[dream.personaId] = [];
                byPersona[dream.personaId].push(dream);
            } else {
                ungrouped.push(dream);
            }
        }
        return { byPersona, ungrouped };
    });
    
    // Group journal entries by persona
    let groupedJournalEntries = $derived(() => {
        const byPersona: Record<string, typeof appState.journalEntries> = {};
        const ungrouped: typeof appState.journalEntries = [];
        
        // Filter by search results if active
        let entriesToGroup = appState.journalEntries;
        if (journalSearchResultIds !== null) {
            entriesToGroup = entriesToGroup.filter(e => journalSearchResultIds!.has(e.id));
        }
        // Filter by tag if selected
        if (selectedTagFilter) {
            entriesToGroup = entriesToGroup.filter(e => e.tags?.includes(selectedTagFilter!));
        }
        if (selectedPersonaFilter) {
            entriesToGroup = entriesToGroup.filter(e => e.personaId === selectedPersonaFilter);
        }
        
        for (const entry of entriesToGroup) {
            if (entry.personaId) {
                if (!byPersona[entry.personaId]) byPersona[entry.personaId] = [];
                byPersona[entry.personaId].push(entry);
            } else {
                ungrouped.push(entry);
            }
        }
        return { byPersona, ungrouped };
    });
    
    function formatTime(isoString: string) {
        const date = new Date(isoString);
        const now = new Date();
        const diffMs = now.getTime() - date.getTime();
        const diffMins = Math.floor(diffMs / 60000);
        
        if (diffMins < 1) return 'just now';
        if (diffMins < 60) return `${diffMins}m ago`;
        
        const diffHours = Math.floor(diffMins / 60);
        if (diffHours < 24) return `${diffHours}h ago`;
        
        return date.toLocaleDateString();
    }
    
    function formatJournalDate(dateStr: string) {
        const date = new Date(dateStr);
        return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
    }
    
    function getMoodEmoji(mood?: string) {
        const moods: Record<string, string> = {
            'contemplative': '🌙',
            'reflective': '💭',
            'joyful': '✨',
            'curious': '🔮',
            'serene': '🌸',
            'thoughtful': '📖',
        };
        return moods[mood || ''] || '📝';
    }
    
    function getDreamMoodEmoji(mood?: string) {
        const moods: Record<string, string> = {
            'mystical': '🌌',
            'contemplative': '💭',
            'serene': '🌙',
            'ethereal': '✨',
            'surreal': '🔮',
            'peaceful': '🌸',
        };
        return moods[mood || ''] || '💫';
    }
    
    function setView(tab: TabType) {
        appState.sidebarView = tab;
    }
    
    // Group management functions
    function createGroup() {
        if (newGroupName.trim()) {
            appState.createGroup(newGroupName.trim(), newGroupColor);
            newGroupName = '';
            newGroupColor = '#6b7280';
        }
    }
    
    function startEditGroup(group: typeof appState.chatGroups[0]) {
        editingGroupId = group.id;
        editGroupName = group.name;
        editGroupColor = group.color;
    }
    
    function saveEditGroup() {
        if (editingGroupId && editGroupName.trim()) {
            appState.updateGroup(editingGroupId, { name: editGroupName.trim(), color: editGroupColor });
            editingGroupId = null;
        }
    }
    
    function cancelEditGroup() {
        editingGroupId = null;
    }
    
    // Tag management functions
    function createTag() {
        if (newTagName.trim()) {
            appState.createTag(newTagName.trim(), newTagColor);
            newTagName = '';
            newTagColor = '#6b7280';
        }
    }
    
    function startEditTag(tag: typeof appState.tags[0]) {
        editingTagId = tag.id;
        editTagName = tag.name;
        editTagColor = tag.color;
    }
    
    function saveEditTag() {
        if (editingTagId && editTagName.trim()) {
            appState.updateTag(editingTagId, { name: editTagName.trim(), color: editTagColor });
            editingTagId = null;
        }
    }
    
    function cancelEditTag() {
        editingTagId = null;
    }
    
    // Context menu for chat assignment
    function showChatContextMenu(e: MouseEvent, chatId: string) {
        e.preventDefault();
        e.stopPropagation();
        chatContextMenuId = chatId;
        contextMenuPosition = { x: e.clientX, y: e.clientY };
    }
    
    function closeChatContextMenu() {
        chatContextMenuId = null;
    }
    
    function assignChatToGroup(chatId: string, groupId: string | undefined) {
        appState.setChatGroup(chatId, groupId);
        closeChatContextMenu();
    }
    
    function toggleChatTag(chatId: string, tagId: string) {
        const chat = appState.chats.find(c => c.id === chatId);
        if (chat?.tags?.includes(tagId)) {
            appState.removeChatTag(chatId, tagId);
        } else {
            appState.addChatTag(chatId, tagId);
        }
    }
    
    function getTagById(tagId: string) {
        return appState.tags.find(t => t.id === tagId);
    }
</script>

<aside class="sidebar" class:is-open={appState.isSidebarOpen} class:collapsed={!appState.isSidebarOpen}>
    
    <!-- Collapse/Expand toggle button -->
    <button class="collapse-toggle" onclick={() => appState.isSidebarOpen = !appState.isSidebarOpen} title={appState.isSidebarOpen ? 'Collapse sidebar' : 'Expand sidebar'}>
        <span class="toggle-icon">{appState.isSidebarOpen ? '◀' : '▶'}</span>
    </button>
    
    <!-- Icon rail (shown when collapsed) -->
    {#if !appState.isSidebarOpen}
        <div class="icon-rail">
            <a 
                class="icon-btn"
                class:active={appState.sidebarView === 'history' && !isCanvasPage}
                href="/"
                onclick={() => { appState.sidebarView = 'history'; }}
                title="Chat History"
            >💬</a>
            <a 
                class="icon-btn"
                class:active={isCanvasPage && appState.sidebarView === 'history'}
                href="/canvas"
                onclick={() => { appState.sidebarView = 'history'; }}
                title="Canvas - Image Generation"
            >🎨</a>
            <button 
                class="icon-btn"
                class:active={appState.sidebarView === 'dreams'}
                onclick={() => { appState.sidebarView = 'dreams'; appState.isSidebarOpen = true; }}
                title="Dreams"
            >✨</button>
            <button 
                class="icon-btn"
                class:active={appState.sidebarView === 'personas'}
                onclick={() => { appState.sidebarView = 'personas'; appState.isSidebarOpen = true; }}
                title="Personas"
            >🎭</button>
            <button 
                class="icon-btn"
                class:active={appState.sidebarView === 'journal'}
                onclick={() => { appState.sidebarView = 'journal'; appState.isSidebarOpen = true; }}
                title="Journal"
            >📔</button>
            <button 
                class="icon-btn"
                class:active={appState.sidebarView === 'logs'}
                onclick={() => { appState.sidebarView = 'logs'; appState.isSidebarOpen = true; }}
                title="Logs"
            >📋</button>
            <button 
                class="icon-btn"
                class:active={appState.sidebarView === 'settings'}
                onclick={() => { appState.sidebarView = 'settings'; appState.isSidebarOpen = true; }}
                title="Settings"
            >⚙️</button>
        </div>
    {/if}
    
    <!-- Full sidebar content (shown when expanded) -->
    {#if appState.isSidebarOpen}
    <!-- Header with tabs -->
    <div class="sidebar-header">
        <div class="tab-buttons">
            <a 
                class="tab-btn"
                class:active={appState.sidebarView === 'history' && !isCanvasPage}
                href="/"
                onclick={() => { appState.sidebarView = 'history'; }}
                title="Chat History"
            >
                💬 Chats
            </a>
            <a 
                class="tab-btn"
                class:active={isCanvasPage && appState.sidebarView === 'history'}
                href="/canvas"
                onclick={() => { appState.sidebarView = 'history'; }}
                title="Canvas - Image Generation"
            >
                🎨 Canvas
            </a>
            <button 
                class="tab-btn"
                class:active={appState.sidebarView === 'dreams'}
                onclick={() => setView('dreams')}
                title="Dreams"
            >
                ✨ Dreams
            </button>
            <button 
                class="tab-btn"
                class:active={appState.sidebarView === 'personas'}
                onclick={() => setView('personas')}
                title="Personas"
            >
                🎭 Personas
            </button>
            <button 
                class="tab-btn"
                class:active={appState.sidebarView === 'journal'}
                onclick={() => setView('journal')}
                title="Journal"
            >
                📔 Journal
            </button>
            <button 
                class="tab-btn"
                class:active={appState.sidebarView === 'logs'}
                onclick={() => setView('logs')}
                title="System Logs"
            >
                📋 Logs
            </button>
            <button 
                class="tab-btn"
                class:active={appState.sidebarView === 'settings'}
                onclick={() => setView('settings')}
                title="Settings"
            >
                ⚙️ Config
            </button>
        </div>
    </div>
    
    <!-- Content Area -->
    <div class="scroll-area">
        
        <!-- History Tab -->
        {#if appState.sidebarView === 'history'}
            <div class="tab-content">
                <button class="new-chat-btn" onclick={() => appState.createNewChat()}>
                    <span>+ New Chat</span>
                </button>
                
                <!-- Search Bar -->
                <div class="chat-search">
                    <span class="search-icon">🔍</span>
                    <input 
                        type="text" 
                        class="search-input" 
                        placeholder="Search chats..." 
                        value={searchQuery}
                        oninput={(e) => handleSearchInput((e.target as HTMLInputElement).value)}
                    />
                    {#if searchQuery}
                        <button class="search-clear" onclick={() => handleSearchInput('')}>✕</button>
                    {/if}
                    {#if isSearching}
                        <span class="search-spinner">⟳</span>
                    {/if}
                </div>
                
                <!-- Management buttons -->
                <div class="management-buttons">
                    <button class="mgmt-btn" class:active={showGroupManager} onclick={() => { showGroupManager = !showGroupManager; showTagManager = false; }}>
                        📁 Groups
                    </button>
                    <button class="mgmt-btn" class:active={showTagManager} onclick={() => { showTagManager = !showTagManager; showGroupManager = false; }}>
                        🏷️ Tags
                    </button>
                </div>
                
                <!-- Group Manager Panel -->
                {#if showGroupManager}
                    <div class="manager-panel">
                        <div class="manager-header">Manage Groups</div>
                        <div class="manager-form">
                            <input type="text" placeholder="New group name..." bind:value={newGroupName} class="manager-input" />
                            <ColorPicker bind:value={newGroupColor} />
                            <button class="manager-add-btn" onclick={createGroup}>+</button>
                        </div>
                        <div class="manager-list">
                            {#each appState.chatGroups as group (group.id)}
                                <div class="manager-item">
                                    {#if editingGroupId === group.id}
                                        <input type="text" bind:value={editGroupName} class="manager-input" />
                                        <ColorPicker bind:value={editGroupColor} />
                                        <button class="manager-save-btn" onclick={saveEditGroup}>✓</button>
                                        <button class="manager-cancel-btn" onclick={cancelEditGroup}>✕</button>
                                    {:else}
                                        <span class="group-color-dot" style="background: {group.color}"></span>
                                        <span class="manager-item-name">{group.name}</span>
                                        <button class="manager-edit-btn" onclick={() => startEditGroup(group)}>✎</button>
                                        {#if group.id !== 'default'}
                                            <button class="manager-delete-btn" onclick={() => appState.deleteGroup(group.id)}>✕</button>
                                        {/if}
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    </div>
                {/if}
                
                <!-- Tag Manager Panel -->
                {#if showTagManager}
                    <div class="manager-panel">
                        <div class="manager-header">Manage Tags</div>
                        <div class="manager-form">
                            <input type="text" placeholder="New tag name..." bind:value={newTagName} class="manager-input" />
                            <ColorPicker bind:value={newTagColor} />
                            <button class="manager-add-btn" onclick={createTag}>+</button>
                        </div>
                        <div class="manager-list">
                            {#each appState.tags as tag (tag.id)}
                                <div class="manager-item">
                                    {#if editingTagId === tag.id}
                                        <input type="text" bind:value={editTagName} class="manager-input" />
                                        <ColorPicker bind:value={editTagColor} />
                                        <button class="manager-save-btn" onclick={saveEditTag}>✓</button>
                                        <button class="manager-cancel-btn" onclick={cancelEditTag}>✕</button>
                                    {:else}
                                        <span class="tag-badge-mini" style="background: {tag.color}">{tag.name}</span>
                                        <button class="manager-edit-btn" onclick={() => startEditTag(tag)}>✎</button>
                                        <button class="manager-delete-btn" onclick={() => appState.deleteTag(tag.id)}>✕</button>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    </div>
                {/if}
                
                <!-- Tag Filter -->
                {#if appState.tags.length > 0}
                    <div class="tag-filter">
                        <button 
                            class="tag-filter-btn" 
                            class:active={!selectedTagFilter}
                            onclick={() => selectedTagFilter = null}
                        >All</button>
                        {#each appState.tags as tag (tag.id)}
                            <button 
                                class="tag-filter-btn"
                                class:active={selectedTagFilter === tag.id}
                                style="--tag-color: {tag.color}"
                                onclick={() => selectedTagFilter = selectedTagFilter === tag.id ? null : tag.id}
                            >{tag.name}</button>
                        {/each}
                    </div>
                {/if}
                
                <div class="section-title">Chat Sessions</div>
                
                {#if appState.chats.length === 0}
                    <div class="empty-state">No chats yet. Create a new one!</div>
                {:else if searchResultIds !== null && searchResultIds.size === 0}
                    <div class="empty-state">No chats match your search.</div>
                {:else}
                    <!-- Grouped Chats -->
                    {#each appState.chatGroups as group (group.id)}
                        {@const groupChats = groupedChats().groups[group.id] || []}
                        {#if groupChats.length > 0}
                            <div class="chat-group">
                                <button class="group-header" onclick={() => appState.toggleGroupCollapse(group.id)}>
                                    <span class="group-collapse-icon">{group.collapsed ? '▶' : '▼'}</span>
                                    <span class="group-color-dot" style="background: {group.color}"></span>
                                    <span class="group-name">{group.name}</span>
                                    <span class="group-count">({groupChats.length})</span>
                                </button>
                                {#if !group.collapsed}
                                    <div class="group-chats">
                                        {#each groupChats as chat (chat.id)}
                                            {@const branches = chat.branches || []}
                                            {@const currentBranch = branches.find(b => b.id === chat.currentBranchId) || branches[0]}
                                            <div class="chat-session" class:active={appState.currentChatId === chat.id}>
                                                <button 
                                                    class="chat-session-btn" 
                                                    onclick={() => appState.loadChat(chat.id)}
                                                    oncontextmenu={(e) => showChatContextMenu(e, chat.id)}
                                                >
                                                    <div class="chat-header">
                                                        <span class="chat-title">{chat.title}</span>
                                                        <span class="chat-count">{currentBranch?.messages?.length || 0}{branches.length > 1 ? ` (${branches.length} branches)` : ''}</span>
                                                    </div>
                                                    {#if chat.tags && chat.tags.length > 0}
                                                        <div class="chat-tags">
                                                            {#each chat.tags as tagId}
                                                                {@const tag = getTagById(tagId)}
                                                                {#if tag}
                                                                    <span class="chat-tag-dot" style="background: {tag.color}" title={tag.name}></span>
                                                                {/if}
                                                            {/each}
                                                        </div>
                                                    {/if}
                                                    <span class="chat-date">{formatTime(chat.createdAt)}</span>
                                                </button>
                                                <button class="chat-delete-btn" onclick={() => appState.deleteChat(chat.id)} title="Delete chat">
                                                    ✕
                                                </button>
                                            </div>
                                        {/each}
                                    </div>
                                {/if}
                            </div>
                        {/if}
                    {/each}
                    
                    <!-- Ungrouped Chats -->
                    {#if groupedChats().ungrouped.length > 0}
                        <div class="chat-group ungrouped">
                            <div class="group-header-static">
                                <span class="group-name">📥 Ungrouped</span>
                                <span class="group-count">({groupedChats().ungrouped.length})</span>
                            </div>
                            <div class="group-chats">
                                {#each groupedChats().ungrouped as chat (chat.id)}
                                    {@const branches = chat.branches || []}
                                    {@const currentBranch = branches.find(b => b.id === chat.currentBranchId) || branches[0]}
                                    <div class="chat-session" class:active={appState.currentChatId === chat.id}>
                                        <button 
                                            class="chat-session-btn" 
                                            onclick={() => appState.loadChat(chat.id)}
                                            oncontextmenu={(e) => showChatContextMenu(e, chat.id)}
                                        >
                                            <div class="chat-header">
                                                <span class="chat-title">{chat.title}</span>
                                                <span class="chat-count">{currentBranch?.messages?.length || 0}{branches.length > 1 ? ` (${branches.length} branches)` : ''}</span>
                                            </div>
                                            {#if chat.tags && chat.tags.length > 0}
                                                <div class="chat-tags">
                                                    {#each chat.tags as tagId}
                                                        {@const tag = getTagById(tagId)}
                                                        {#if tag}
                                                            <span class="chat-tag-dot" style="background: {tag.color}" title={tag.name}></span>
                                                        {/if}
                                                    {/each}
                                                </div>
                                            {/if}
                                            <span class="chat-date">{formatTime(chat.createdAt)}</span>
                                        </button>
                                        <button class="chat-delete-btn" onclick={() => appState.deleteChat(chat.id)} title="Delete chat">
                                            ✕
                                        </button>
                                    </div>
                                {/each}
                            </div>
                        </div>
                    {/if}
                {/if}
            </div>
        {/if}
        
        <!-- Personas Tab -->
        {#if appState.sidebarView === 'personas'}
            <div class="tab-content">
                <button class="new-chat-btn" onclick={() => appState.openPersonaEditor()}>
                    <span>+ New Persona</span>
                </button>
                
                <!-- AI Personas -->
                <div class="section-title">🤖 AI Personas</div>
                {#if appState.aiPersonas.length === 0}
                    <div class="empty-state">No AI personas yet</div>
                {:else}
                    {#each appState.aiPersonas as persona (persona.id)}
                        <div class="persona-item" class:active={appState.currentAiPersonaId === persona.id}>
                            <button class="persona-btn" onclick={() => appState.currentAiPersonaId = persona.id}>
                                <div class="persona-name">{persona.name}</div>
                                <div class="persona-desc">{persona.description}</div>
                            </button>
                            <button class="edit-btn" onclick={() => appState.openPersonaEditor(persona)} title="Edit">
                                ✎
                            </button>
                        </div>
                    {/each}
                {/if}
                
                <!-- User Personas -->
                <div class="section-title" style="margin-top: 1.5rem;">👤 User Personas</div>
                {#if appState.userPersonas.length === 0}
                    <div class="empty-state">No user personas yet</div>
                {:else}
                    {#each appState.userPersonas as persona (persona.id)}
                        <div class="persona-item" class:active={appState.currentUserPersonaId === persona.id}>
                            <button class="persona-btn" onclick={() => appState.currentUserPersonaId = persona.id}>
                                <div class="persona-name">{persona.name}</div>
                                <div class="persona-desc">{persona.description}</div>
                            </button>
                            <button class="edit-btn" onclick={() => appState.openPersonaEditor(persona)} title="Edit">
                                ✎
                            </button>
                        </div>
                    {/each}
                {/if}
            </div>
        {/if}
        
        <!-- Dreams Tab -->
        {#if appState.sidebarView === 'dreams'}
            <div class="tab-content">
                <div class="section-title">Dreams & Visions</div>
                <p class="section-description">Visions from periods of idle contemplation</p>
                
                <!-- Search Bar -->
                <div class="chat-search">
                    <span class="search-icon">🔍</span>
                    <input 
                        type="text" 
                        class="search-input" 
                        placeholder="Search dreams..." 
                        value={dreamSearchQuery}
                        oninput={(e) => handleDreamSearchInput((e.target as HTMLInputElement).value)}
                    />
                    {#if dreamSearchQuery}
                        <button class="search-clear" onclick={() => handleDreamSearchInput('')}>✕</button>
                    {/if}
                    {#if isDreamSearching}
                        <span class="search-spinner">⟳</span>
                    {/if}
                </div>
                
                <!-- Persona Filter -->
                <div class="filter-bar">
                    <button 
                        class="filter-btn" 
                        class:active={!selectedPersonaFilter}
                        onclick={() => selectedPersonaFilter = null}
                    >
                        All
                    </button>
                    {#each allPersonas as persona (persona.id)}
                        <button 
                            class="filter-btn persona-filter" 
                            class:active={selectedPersonaFilter === persona.id}
                            onclick={() => selectedPersonaFilter = selectedPersonaFilter === persona.id ? null : persona.id}
                        >
                            <span class="persona-avatar">{persona.avatar || '🤖'}</span>
                            {persona.name}
                        </button>
                    {/each}
                </div>
                
                {#if appState.dreams.length === 0}
                    <div class="empty-state">No dreams recorded yet</div>
                {:else}
                    <!-- Grouped by Persona -->
                    {#each Object.entries(groupedDreams().byPersona) as [personaId, dreams] (personaId)}
                        {@const persona = allPersonas.find(p => p.id === personaId)}
                        <div class="persona-group">
                            <div class="persona-group-header">
                                <span class="persona-avatar">{persona?.avatar || '💫'}</span>
                                <span class="persona-group-name">{persona?.name || 'Unknown'}'s Dreams</span>
                                <span class="group-count">{dreams.length}</span>
                            </div>
                            <div class="group-items">
                                {#each dreams as dream (dream.id)}
                                    <button class="dream-item" onclick={() => appState.openDream(dream)}>
                                        <div class="dream-header">
                                            <span class="dream-title">{dream.title}</span>
                                            <span class="dream-mood">{getDreamMoodEmoji(dream.mood)}</span>
                                        </div>
                                        <div class="dream-meta">
                                            <span class="dream-time">{formatTime(dream.timestamp)}</span>
                                            {#if dream.tags && dream.tags.length > 0}
                                                <div class="item-tags">
                                                    {#each dream.tags as tagId}
                                                        {@const tag = appState.tags.find(t => t.id === tagId)}
                                                        {#if tag}
                                                            <span class="tag-dot" style="background: {tag.color}" title={tag.name}></span>
                                                        {/if}
                                                    {/each}
                                                </div>
                                            {/if}
                                        </div>
                                    </button>
                                {/each}
                            </div>
                        </div>
                    {/each}
                    
                    <!-- Ungrouped Dreams -->
                    {#if groupedDreams().ungrouped.length > 0}
                        <div class="persona-group">
                            <div class="persona-group-header">
                                <span class="persona-avatar">💫</span>
                                <span class="persona-group-name">Unattributed</span>
                                <span class="group-count">{groupedDreams().ungrouped.length}</span>
                            </div>
                            <div class="group-items">
                                {#each groupedDreams().ungrouped as dream (dream.id)}
                                    <button class="dream-item" onclick={() => appState.openDream(dream)}>
                                        <div class="dream-header">
                                            <span class="dream-title">{dream.title}</span>
                                            <span class="dream-mood">{getDreamMoodEmoji(dream.mood)}</span>
                                        </div>
                                        <span class="dream-time">{formatTime(dream.timestamp)}</span>
                                    </button>
                                {/each}
                            </div>
                        </div>
                    {/if}
                {/if}
            </div>
        {/if}
        
        <!-- Journal Tab -->
        {#if appState.sidebarView === 'journal'}
            <div class="tab-content">
                <div class="section-title">Journal Entries</div>
                <p class="section-description">Daily reflections from AI personas</p>
                
                <!-- Search Bar -->
                <div class="chat-search">
                    <span class="search-icon">🔍</span>
                    <input 
                        type="text" 
                        class="search-input" 
                        placeholder="Search journal..." 
                        value={journalSearchQuery}
                        oninput={(e) => handleJournalSearchInput((e.target as HTMLInputElement).value)}
                    />
                    {#if journalSearchQuery}
                        <button class="search-clear" onclick={() => handleJournalSearchInput('')}>✕</button>
                    {/if}
                    {#if isJournalSearching}
                        <span class="search-spinner">⟳</span>
                    {/if}
                </div>
                
                <!-- Persona Filter -->
                <div class="filter-bar">
                    <button 
                        class="filter-btn" 
                        class:active={!selectedPersonaFilter}
                        onclick={() => selectedPersonaFilter = null}
                    >
                        All
                    </button>
                    {#each allPersonas as persona (persona.id)}
                        <button 
                            class="filter-btn persona-filter" 
                            class:active={selectedPersonaFilter === persona.id}
                            onclick={() => selectedPersonaFilter = selectedPersonaFilter === persona.id ? null : persona.id}
                        >
                            <span class="persona-avatar">{persona.avatar || '🤖'}</span>
                            {persona.name}
                        </button>
                    {/each}
                </div>
                
                {#if appState.journalEntries.length === 0}
                    <div class="empty-state">No journal entries yet</div>
                {:else}
                    <!-- Grouped by Persona -->
                    {#each Object.entries(groupedJournalEntries().byPersona) as [personaId, entries] (personaId)}
                        {@const persona = allPersonas.find(p => p.id === personaId)}
                        <div class="persona-group">
                            <div class="persona-group-header">
                                <span class="persona-avatar">{persona?.avatar || '📖'}</span>
                                <span class="persona-group-name">{persona?.name || 'Unknown'}'s Journal</span>
                                <span class="group-count">{entries.length}</span>
                            </div>
                            <div class="group-items">
                                {#each entries as entry (entry.id)}
                                    <button class="journal-entry" onclick={() => appState.openJournalEntry(entry)}>
                                        <div class="journal-header">
                                            <span class="journal-title">{entry.title}</span>
                                            <span class="journal-mood">{getMoodEmoji(entry.mood)}</span>
                                        </div>
                                        <div class="journal-meta">
                                            <span class="journal-date">{formatJournalDate(entry.date)}</span>
                                            {#if entry.tags && entry.tags.length > 0}
                                                <div class="item-tags">
                                                    {#each entry.tags as tagId}
                                                        {@const tag = appState.tags.find(t => t.id === tagId)}
                                                        {#if tag}
                                                            <span class="tag-dot" style="background: {tag.color}" title={tag.name}></span>
                                                        {/if}
                                                    {/each}
                                                </div>
                                            {/if}
                                        </div>
                                    </button>
                                {/each}
                            </div>
                        </div>
                    {/each}
                    
                    <!-- Ungrouped Entries -->
                    {#if groupedJournalEntries().ungrouped.length > 0}
                        <div class="persona-group">
                            <div class="persona-group-header">
                                <span class="persona-avatar">📖</span>
                                <span class="persona-group-name">Unattributed</span>
                                <span class="group-count">{groupedJournalEntries().ungrouped.length}</span>
                            </div>
                            <div class="group-items">
                                {#each groupedJournalEntries().ungrouped as entry (entry.id)}
                                    <button class="journal-entry" onclick={() => appState.openJournalEntry(entry)}>
                                        <div class="journal-header">
                                            <span class="journal-title">{entry.title}</span>
                                            <span class="journal-mood">{getMoodEmoji(entry.mood)}</span>
                                        </div>
                                        <span class="journal-date">{formatJournalDate(entry.date)}</span>
                                    </button>
                                {/each}
                            </div>
                        </div>
                    {/if}
                {/if}
            </div>
        {/if}
        
        <!-- Logs Tab -->
        {#if appState.sidebarView === 'logs'}
            <div class="tab-content">
                <div class="section-title">System Logs</div>
                {#each appState.logs.slice().reverse() as log (log.id)}
                    <div class="log-item" class:level-error={log.level === 'error'} class:level-warn={log.level === 'warn'} class:level-debug={log.level === 'debug'}>
                        <div class="log-header">
                            <span class="log-level">[{log.level.toUpperCase()}]</span>
                            <span class="log-time">{formatTime(log.timestamp)}</span>
                        </div>
                        <p class="log-message">{log.message}</p>
                    </div>
                {/each}
                {#if appState.logs.length === 0}
                    <div class="empty-state">No logs available</div>
                {/if}
            </div>
        {/if}
        
        <!-- Settings Tab -->
        {#if appState.sidebarView === 'settings'}
            <div class="tab-content">
                <div class="section-title">Configuration</div>
                
                <!-- Model Selection -->
                <div class="setting-group">
                    <span class="setting-label">Default Model</span>
                    {#if appState.isLoadingModels}
                        <div class="setting-hint">Loading models...</div>
                    {:else if appState.availableModels.length === 0}
                        <div class="setting-hint">No models available</div>
                    {:else}
                        <select bind:value={appState.selectedModel} onchange={() => appState.savePreferences()} class="setting-input">
                            {#each appState.availableModels as model (model.id)}
                                <option value={model.id}>{model.name}</option>
                            {/each}
                        </select>
                    {/if}
                    <div class="setting-hint">Model used for new conversations</div>
                </div>
                
                <!-- Show Thinking -->
                <div class="setting-group">
                    <div class="setting-toggle-row">
                        <span class="setting-label">Show Thinking</span>
                        <button 
                            class="toggle-switch"
                            class:active={appState.showThinking}
                            onclick={() => { appState.showThinking = !appState.showThinking; appState.savePreferences(); }}
                            role="switch"
                            aria-checked={appState.showThinking}
                            aria-label="Toggle show thinking"
                        >
                            <span class="toggle-knob"></span>
                        </button>
                    </div>
                    <div class="setting-hint">Display AI reasoning blocks in chat</div>
                </div>
                
                <!-- Send on Enter -->
                <div class="setting-group">
                    <div class="setting-toggle-row">
                        <span class="setting-label">Send on Enter</span>
                        <button 
                            class="toggle-switch"
                            class:active={appState.sendOnEnter}
                            onclick={() => { appState.sendOnEnter = !appState.sendOnEnter; appState.savePreferences(); }}
                            role="switch"
                            aria-checked={appState.sendOnEnter}
                            aria-label="Toggle send on enter"
                        >
                            <span class="toggle-knob"></span>
                        </button>
                    </div>
                    <div class="setting-hint">{appState.sendOnEnter ? 'Enter sends, Shift+Enter for newline' : 'Ctrl+Enter sends, Enter for newline'}</div>
                </div>
                
                <!-- Danger Zone -->
                <div class="setting-divider"></div>
                <div class="setting-group">
                    <span class="setting-label danger-label">Danger Zone</span>
                    <button class="clear-btn" onclick={() => appState.clearMessages()}>Clear Current Chat</button>
                </div>
                
                <!-- Session -->
                <div class="setting-footer">
                    <span class="setting-footer-text">Session {appState.sessionId.substring(0, 8)}</span>
                </div>
            </div>
        {/if}
        
    </div>
    {/if}
</aside>

<!-- Context Menu for Chat -->
{#if chatContextMenuId}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="context-menu-overlay" onclick={closeChatContextMenu} role="presentation"></div>
    <div class="context-menu" style="left: {contextMenuPosition.x}px; top: {contextMenuPosition.y}px;">
        <div class="context-menu-section">
            <div class="context-menu-title">Move to Group</div>
            <button class="context-menu-item" onclick={() => assignChatToGroup(chatContextMenuId!, undefined)}>
                📥 Ungrouped
            </button>
            {#each appState.chatGroups as group (group.id)}
                <button class="context-menu-item" onclick={() => assignChatToGroup(chatContextMenuId!, group.id)}>
                    <span class="group-color-dot" style="background: {group.color}"></span>
                    {group.name}
                </button>
            {/each}
        </div>
        <div class="context-menu-divider"></div>
        <div class="context-menu-section">
            <div class="context-menu-title">Tags</div>
            {#each appState.tags as tag (tag.id)}
                {@const chat = appState.chats.find(c => c.id === chatContextMenuId)}
                {@const isTagged = chat?.tags?.includes(tag.id)}
                <button class="context-menu-item" class:tagged={isTagged} onclick={() => toggleChatTag(chatContextMenuId!, tag.id)}>
                    <span class="tag-check">{isTagged ? '✓' : ''}</span>
                    <span class="tag-badge-mini" style="background: {tag.color}">{tag.name}</span>
                </button>
            {/each}
        </div>
    </div>
{/if}

<style>
    /* Theme variables - Dark Reader inspired */
    aside {
        --sb-bg: #1c1e1f;           /* dark gray */
        --sb-border: #2a2c2d;       /* border color */
        --sb-text-main: #e8eaec;    /* light text */
        --sb-text-dim: #8b9295;     /* dim text */
        --sb-accent: #e879f9;       /* magenta accent */
        --sb-hover: rgba(232, 121, 249, 0.1);
    }

    .sidebar {
        background-color: rgba(28, 30, 31, 0.95);
        border-right: 1px solid var(--sb-border);
        display: flex;
        flex-direction: column;
        height: 100vh;
        width: 3.5rem; /* Collapsed width for icons */
        overflow: hidden;
        flex-shrink: 0;
        backdrop-filter: blur(10px);
        position: fixed;
        left: 0;
        top: 0;
        z-index: 30;
        transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    }

    .sidebar.is-open {
        width: 18rem; /* w-72 */
    }
    
    .collapse-toggle {
        position: absolute;
        top: 0.75rem;
        right: -0.75rem;
        width: 1.5rem;
        height: 1.5rem;
        background: rgba(35, 37, 38, 0.95);
        border: 1px solid rgba(232, 121, 249, 0.4);
        border-radius: 50%;
        color: rgba(139, 146, 149, 0.9);
        font-size: 0.625rem;
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 40;
        transition: all 0.2s;
    }
    
    .collapse-toggle:hover {
        background: rgba(232, 121, 249, 0.2);
        color: rgba(232, 234, 236, 0.95);
        transform: scale(1.1);
    }
    
    .icon-rail {
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 3rem 0.5rem 0.5rem;
        gap: 0.5rem;
    }
    
    .icon-btn {
        width: 2.5rem;
        height: 2.5rem;
        display: flex;
        align-items: center;
        justify-content: center;
        background: transparent;
        border: 1px solid transparent;
        border-radius: 0.5rem;
        font-size: 1.25rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .icon-btn:hover {
        background: rgba(232, 121, 249, 0.15);
        border-color: rgba(232, 121, 249, 0.3);
    }
    
    .icon-btn.active {
        background: rgba(232, 121, 249, 0.2);
        border-color: rgba(232, 121, 249, 0.4);
    }

    .sidebar-header {
        padding: 0.75rem;
        border-bottom: 1px solid var(--sb-border);
        flex-shrink: 0;
        background-color: rgba(28, 30, 31, 0.5);
        backdrop-filter: blur(10px);
    }

    .tab-buttons {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 0.5rem;
    }

    .tab-btn {
        padding: 0.5rem;
        font-size: 0.75rem;
        font-weight: 600;
        color: var(--sb-text-dim);
        background: transparent;
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.5rem;
        cursor: pointer;
        transition: all 0.2s;
        white-space: nowrap;
        text-decoration: none;
        text-align: center;
    }

    .tab-btn:hover {
        background: var(--sb-hover);
        color: var(--sb-text-main);
        border-color: rgba(232, 121, 249, 0.4);
    }

    .tab-btn.active {
        background: rgba(232, 121, 249, 0.2);
        color: var(--sb-text-main);
        border-color: var(--sb-accent);
    }

    .scroll-area {
        flex: 1;
        overflow-y: auto;
        padding: 0.75rem;
    }

    .tab-content {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    .section-title {
        font-size: 0.875rem;
        font-weight: 700;
        color: var(--sb-text-main);
        padding: 0.5rem 0;
        border-bottom: 1px solid rgba(232, 121, 249, 0.2);
    }

    .section-description {
        font-size: 0.75rem;
        color: var(--sb-text-dim);
        margin-top: -0.75rem;
    }

    /* History Items - OLD (REMOVE) */
    /* Replaced with new chat session styles below */

    /* New Chat Session Styles */
    .new-chat-btn {
        width: 100%;
        padding: 0.75rem;
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.2), rgba(232, 121, 249, 0.1));
        border: 1px solid rgba(232, 121, 249, 0.3);
        color: var(--sb-text-main);
        border-radius: 0.5rem;
        font-weight: 600;
        font-size: 0.85rem;
        cursor: pointer;
        transition: all 0.2s;
        margin-bottom: 1rem;
    }

    .new-chat-btn:hover {
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.3), rgba(232, 121, 249, 0.2));
        border-color: rgba(232, 121, 249, 0.5);
    }

    .new-chat-btn:active {
        transform: scale(0.98);
    }

    .chat-session {
        display: flex;
        gap: 0.5rem;
        padding: 0;
        margin-bottom: 0.5rem;
        border-radius: 0.5rem;
        transition: all 0.2s;
        background: rgba(232, 121, 249, 0.05);
        border: 1px solid rgba(232, 121, 249, 0.1);
    }

    .chat-session.active {
        background: rgba(232, 121, 249, 0.15);
        border-color: rgba(232, 121, 249, 0.4);
    }

    .chat-session-btn {
        flex: 1;
        padding: 0.75rem;
        text-align: left;
        background: transparent;
        border: none;
        color: var(--sb-text-main);
        cursor: pointer;
        transition: all 0.2s;
        border-radius: 0.4rem;
    }

    .chat-session:hover .chat-session-btn {
        background: rgba(232, 121, 249, 0.05);
    }

    .chat-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 0.25rem;
    }

    .chat-title {
        font-size: 0.85rem;
        font-weight: 600;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        flex: 1;
    }

    .chat-count {
        font-size: 0.7rem;
        color: var(--sb-text-dim);
        background: rgba(232, 121, 249, 0.2);
        padding: 0.2rem 0.4rem;
        border-radius: 0.25rem;
        flex-shrink: 0;
    }

    .chat-date {
        font-size: 0.7rem;
        color: var(--sb-text-dim);
        display: block;
    }

    .chat-delete-btn {
        flex-shrink: 0;
        width: 2rem;
        height: 2.5rem;
        padding: 0;
        background: transparent;
        border: none;
        color: rgba(239, 68, 68, 0.6);
        cursor: pointer;
        font-size: 1rem;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.2s;
        border-radius: 0 0.5rem 0.5rem 0;
    }

    .chat-delete-btn:hover {
        background: rgba(239, 68, 68, 0.2);
        color: rgba(239, 68, 68, 1);
    }

    /* Dream Items */
    .dream-item {
        width: 100%;
        padding: 0.75rem;
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.08), rgba(192, 80, 210, 0.04));
        border: 1px solid rgba(232, 121, 249, 0.18);
        border-radius: 0.5rem;
        cursor: pointer;
        text-align: left;
        transition: all 0.2s;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    
    .dream-item:hover {
        background: linear-gradient(135deg, rgba(232, 121, 249, 0.15), rgba(192, 80, 210, 0.08));
        border-color: rgba(232, 121, 249, 0.35);
        transform: translateX(2px);
        box-shadow: 0 0 15px rgba(232, 121, 249, 0.12);
    }
    
    .dream-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 0.5rem;
    }
    
    .dream-title {
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--sb-text-main);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    
    .dream-mood {
        font-size: 1rem;
        flex-shrink: 0;
    }

    .dream-time {
        font-size: 0.7rem;
        color: var(--sb-text-dim);
    }

    /* Journal Items */
    .journal-entry {
        width: 100%;
        padding: 0.75rem;
        background: rgba(232, 121, 249, 0.06);
        border: 1px solid rgba(232, 121, 249, 0.15);
        border-radius: 0.5rem;
        cursor: pointer;
        text-align: left;
        transition: all 0.2s;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    
    .journal-entry:hover {
        background: rgba(232, 121, 249, 0.12);
        border-color: rgba(232, 121, 249, 0.3);
        transform: translateX(2px);
    }
    
    .journal-header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 0.5rem;
    }
    
    .journal-title {
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--sb-text-main);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    
    .journal-mood {
        font-size: 1rem;
        flex-shrink: 0;
    }
    
    .journal-date {
        font-size: 0.7rem;
        color: var(--sb-text-dim);
    }

    /* Log Items */
    .log-item {
        padding: 0.5rem;
        background: rgba(232, 121, 249, 0.04);
        border: 1px solid rgba(232, 121, 249, 0.1);
        border-radius: 0.4rem;
        font-family: monospace;
        font-size: 0.7rem;
    }

    .log-item.level-error {
        background: rgba(239, 68, 68, 0.1);
        border-color: rgba(239, 68, 68, 0.2);
    }

    .log-item.level-warn {
        background: rgba(245, 158, 11, 0.1);
        border-color: rgba(245, 158, 11, 0.2);
    }

    .log-item.level-debug {
        background: rgba(59, 130, 246, 0.05);
        border-color: rgba(59, 130, 246, 0.1);
    }

    .log-header {
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.25rem;
    }

    .log-level {
        font-weight: 700;
        color: var(--sb-text-main);
    }

    .log-time {
        color: var(--sb-text-dim);
    }

    .log-message {
        margin: 0;
        color: var(--sb-text-main);
    }

    /* Settings */
    .setting-group {
        display: flex;
        flex-direction: column;
        gap: 0.35rem;
    }

    .setting-group .setting-label {
        font-size: 0.8rem;
        font-weight: 600;
        color: var(--sb-text-main);
    }

    .setting-input {
        padding: 0.4rem;
        background: rgba(20, 22, 23, 0.8);
        border: 1px solid rgba(232, 121, 249, 0.2);
        border-radius: 0.4rem;
        color: var(--sb-text-main);
        font-size: 0.8rem;
    }

    .setting-input:focus {
        outline: none;
        border-color: var(--sb-accent);
    }

    .setting-hint {
        font-size: 0.7rem;
        color: var(--sb-text-dim);
        line-height: 1.3;
    }

    .setting-toggle-row {
        display: flex;
        align-items: center;
        justify-content: space-between;
    }

    .toggle-switch {
        position: relative;
        width: 36px;
        height: 20px;
        background: rgba(55, 60, 63, 0.8);
        border: 1px solid rgba(232, 121, 249, 0.15);
        border-radius: 10px;
        cursor: pointer;
        transition: all 0.2s;
        padding: 0;
        flex-shrink: 0;
    }

    .toggle-switch.active {
        background: rgba(232, 121, 249, 0.3);
        border-color: rgba(232, 121, 249, 0.5);
    }

    .toggle-knob {
        position: absolute;
        top: 2px;
        left: 2px;
        width: 14px;
        height: 14px;
        background: var(--sb-text-dim);
        border-radius: 50%;
        transition: all 0.2s;
    }

    .toggle-switch.active .toggle-knob {
        left: 18px;
        background: var(--sb-accent);
    }

    .setting-divider {
        height: 1px;
        background: var(--sb-border);
        margin: 0.5rem 0;
    }

    .danger-label {
        color: rgba(239, 68, 68, 0.8) !important;
    }

    .clear-btn {
        padding: 0.5rem;
        background: rgba(239, 68, 68, 0.15);
        border: 1px solid rgba(239, 68, 68, 0.25);
        color: rgba(254, 202, 202, 0.8);
        border-radius: 0.5rem;
        font-size: 0.8rem;
        font-weight: 600;
        cursor: pointer;
        transition: all 0.2s;
    }

    .clear-btn:hover {
        background: rgba(239, 68, 68, 0.3);
        border-color: rgba(239, 68, 68, 0.5);
    }

    .setting-footer {
        margin-top: auto;
        padding-top: 1rem;
        text-align: center;
    }

    .setting-footer-text {
        font-size: 0.65rem;
        color: var(--sb-text-dim);
        opacity: 0.6;
    }

    /* Empty State */
    .empty-state {
        padding: 1rem;
        text-align: center;
        color: var(--sb-text-dim);
        font-size: 0.8rem;
        font-style: italic;
    }
    
    /* Persona Items */
    .persona-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 0.5rem;
        padding: 0.5rem;
        background: rgba(28, 30, 31, 0.5);
        border: 1px solid transparent;
        border-radius: 0.5rem;
        transition: all 0.2s;
    }
    
    .persona-item:hover {
        background: rgba(232, 121, 249, 0.08);
        border-color: rgba(232, 121, 249, 0.15);
    }
    
    .persona-item.active {
        background: rgba(232, 121, 249, 0.15);
        border-color: rgba(232, 121, 249, 0.3);
    }
    
    .persona-btn {
        flex: 1;
        min-width: 0;
        overflow: hidden;
        background: none;
        border: none;
        text-align: left;
        cursor: pointer;
        padding: 0;
    }
    
    .persona-name {
        font-size: 0.85rem;
        font-weight: 600;
        color: var(--sb-text-main);
        margin-bottom: 0.125rem;
    }
    
    .persona-desc {
        font-size: 0.7rem;
        color: var(--sb-text-dim);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    
    .edit-btn {
        flex-shrink: 0;
        width: 1.5rem;
        height: 1.5rem;
        padding: 0;
        background: rgba(232, 121, 249, 0.1);
        border: 1px solid rgba(232, 121, 249, 0.15);
        border-radius: 0.25rem;
        color: var(--sb-accent);
        cursor: pointer;
        font-size: 0.75rem;
        transition: all 0.2s;
        display: flex;
        align-items: center;
        justify-content: center;
    }
    
    .edit-btn:hover {
        background: rgba(232, 121, 249, 0.2);
        border-color: rgba(232, 121, 249, 0.3);
    }

    /* Scrollbar styling */
    .scroll-area::-webkit-scrollbar {
        width: 6px;
    }

    .scroll-area::-webkit-scrollbar-track {
        background: transparent;
    }

    .scroll-area::-webkit-scrollbar-thumb {
        background: rgba(232, 121, 249, 0.25);
        border-radius: 3px;
    }

    .scroll-area::-webkit-scrollbar-thumb:hover {
        background: rgba(232, 121, 249, 0.4);
    }
    
    /* Group & Tag Management Styles */
    
    /* Chat Search */
    .chat-search {
        display: flex;
        align-items: center;
        gap: 0.375rem;
        padding: 0.4rem 0.6rem;
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid rgba(232, 121, 249, 0.15);
        border-radius: 0.5rem;
        margin-bottom: 0.5rem;
        transition: border-color 0.2s;
    }
    
    .chat-search:focus-within {
        border-color: rgba(232, 121, 249, 0.5);
    }
    
    .search-icon {
        font-size: 0.75rem;
        opacity: 0.5;
        flex-shrink: 0;
    }
    
    .search-input {
        flex: 1;
        background: transparent;
        border: none;
        outline: none;
        color: var(--sb-text-main);
        font-size: 0.8rem;
        min-width: 0;
    }
    
    .search-input::placeholder {
        color: var(--sb-text-dim);
        opacity: 0.6;
    }
    
    .search-clear {
        background: none;
        border: none;
        color: var(--sb-text-dim);
        cursor: pointer;
        font-size: 0.7rem;
        padding: 0.1rem 0.2rem;
        border-radius: 3px;
        transition: all 0.15s;
        flex-shrink: 0;
    }
    
    .search-clear:hover {
        color: var(--sb-accent);
        background: rgba(232, 121, 249, 0.15);
    }
    
    .search-spinner {
        font-size: 0.75rem;
        color: var(--sb-accent);
        animation: spin 1s linear infinite;
        flex-shrink: 0;
    }
    
    @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
    }

    .management-buttons {
        display: flex;
        gap: 0.5rem;
        padding: 0.5rem 0;
        border-bottom: 1px solid var(--sb-border);
        margin-bottom: 0.5rem;
    }
    
    .mgmt-btn {
        flex: 1;
        padding: 0.35rem 0.5rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid transparent;
        border-radius: 4px;
        color: var(--sb-text-dim);
        font-size: 0.75rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .mgmt-btn:hover, .mgmt-btn.active {
        background: rgba(232, 121, 249, 0.15);
        border-color: rgba(232, 121, 249, 0.3);
        color: var(--sb-accent);
    }
    
    .manager-panel {
        background: rgba(0, 0, 0, 0.2);
        border: 1px solid var(--sb-border);
        border-radius: 6px;
        padding: 0.75rem;
        margin-bottom: 0.75rem;
    }
    
    .manager-header {
        font-size: 0.75rem;
        font-weight: 600;
        color: var(--sb-text-main);
        margin-bottom: 0.5rem;
    }
    
    .manager-form {
        display: flex;
        gap: 0.25rem;
        margin-bottom: 0.5rem;
    }
    
    .manager-input {
        flex: 1;
        padding: 0.3rem 0.5rem;
        background: rgba(0, 0, 0, 0.3);
        border: 1px solid var(--sb-border);
        border-radius: 4px;
        color: var(--sb-text-main);
        font-size: 0.75rem;
    }
    
    .manager-input:focus {
        outline: none;
        border-color: var(--sb-accent);
    }
    
    .manager-add-btn, .manager-save-btn, .manager-cancel-btn, .manager-edit-btn, .manager-delete-btn {
        padding: 0.2rem 0.5rem;
        border: 1px solid var(--sb-border);
        border-radius: 4px;
        cursor: pointer;
        font-size: 0.7rem;
        transition: all 0.2s;
    }
    
    .manager-add-btn, .manager-save-btn {
        background: rgba(232, 121, 249, 0.2);
        color: var(--sb-accent);
    }
    
    .manager-add-btn:hover, .manager-save-btn:hover {
        background: rgba(232, 121, 249, 0.35);
    }
    
    .manager-cancel-btn, .manager-delete-btn {
        background: rgba(239, 68, 68, 0.15);
        color: #ef4444;
    }
    
    .manager-cancel-btn:hover, .manager-delete-btn:hover {
        background: rgba(239, 68, 68, 0.3);
    }
    
    .manager-edit-btn {
        background: transparent;
        color: var(--sb-text-dim);
    }
    
    .manager-edit-btn:hover {
        background: rgba(232, 121, 249, 0.15);
        color: var(--sb-accent);
    }
    
    .manager-list {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    
    .manager-item {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0.3rem 0.4rem;
        background: rgba(0, 0, 0, 0.15);
        border-radius: 4px;
    }
    
    .manager-item-name {
        flex: 1;
        font-size: 0.75rem;
        color: var(--sb-text-main);
    }
    
    .group-color-dot {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        flex-shrink: 0;
    }
    
    .tag-badge-mini {
        padding: 0.15rem 0.4rem;
        border-radius: 3px;
        font-size: 0.65rem;
        color: white;
        font-weight: 500;
    }
    
    /* Tag Filter */
    .tag-filter {
        display: flex;
        flex-wrap: wrap;
        gap: 0.25rem;
        padding: 0.5rem 0;
        border-bottom: 1px solid var(--sb-border);
        margin-bottom: 0.5rem;
    }
    
    .tag-filter-btn {
        padding: 0.2rem 0.5rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid transparent;
        border-radius: 3px;
        color: var(--sb-text-dim);
        font-size: 0.65rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .tag-filter-btn:hover {
        background: rgba(232, 121, 249, 0.15);
    }
    
    .tag-filter-btn.active {
        background: var(--tag-color, var(--sb-accent));
        color: white;
        border-color: transparent;
    }
    
    /* Chat Groups */
    .chat-group {
        margin-bottom: 0.5rem;
    }
    
    .group-header {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0.4rem 0.5rem;
        background: rgba(0, 0, 0, 0.2);
        border: none;
        border-radius: 4px;
        cursor: pointer;
        width: 100%;
        text-align: left;
        transition: all 0.2s;
    }
    
    .group-header:hover {
        background: rgba(232, 121, 249, 0.1);
    }
    
    .group-header-static {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0.4rem 0.5rem;
        color: var(--sb-text-dim);
    }
    
    .group-collapse-icon {
        font-size: 0.6rem;
        color: var(--sb-text-dim);
        width: 12px;
    }
    
    .group-name {
        flex: 1;
        font-size: 0.75rem;
        font-weight: 500;
        color: var(--sb-text-main);
    }
    
    .group-count {
        font-size: 0.65rem;
        color: var(--sb-text-dim);
    }
    
    .group-chats {
        padding-left: 0.75rem;
        border-left: 2px solid rgba(232, 121, 249, 0.2);
        margin-left: 0.5rem;
        margin-top: 0.25rem;
    }
    
    /* Chat Tags */
    .chat-tags {
        display: flex;
        gap: 0.2rem;
        margin-top: 0.15rem;
    }
    
    .chat-tag-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
    }
    
    /* Context Menu */
    .context-menu-overlay {
        position: fixed;
        top: 0;
        left: 0;
        width: 100vw;
        height: 100vh;
        z-index: 999;
    }
    
    .context-menu {
        position: fixed;
        background: var(--sb-bg);
        border: 1px solid var(--sb-border);
        border-radius: 6px;
        padding: 0.5rem 0;
        min-width: 180px;
        z-index: 1000;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    }
    
    .context-menu-section {
        padding: 0.25rem 0;
    }
    
    .context-menu-title {
        font-size: 0.65rem;
        font-weight: 600;
        color: var(--sb-text-dim);
        padding: 0.25rem 0.75rem;
        text-transform: uppercase;
        letter-spacing: 0.5px;
    }
    
    .context-menu-divider {
        height: 1px;
        background: var(--sb-border);
        margin: 0.25rem 0;
    }
    
    .context-menu-item {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        width: 100%;
        padding: 0.4rem 0.75rem;
        background: none;
        border: none;
        text-align: left;
        color: var(--sb-text-main);
        font-size: 0.75rem;
        cursor: pointer;
        transition: background 0.15s;
    }
    
    .context-menu-item:hover {
        background: rgba(232, 121, 249, 0.15);
    }
    
    .context-menu-item.tagged {
        background: rgba(232, 121, 249, 0.08);
    }
    
    .tag-check {
        width: 14px;
        color: var(--sb-accent);
        font-size: 0.7rem;
    }
    
    /* Persona Filter Bar */
    .filter-bar {
        display: flex;
        flex-wrap: wrap;
        gap: 0.375rem;
        padding: 0.5rem 0;
        margin-bottom: 0.75rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.1);
    }
    
    .filter-btn {
        display: flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.25rem 0.5rem;
        background: rgba(232, 121, 249, 0.08);
        border: 1px solid rgba(232, 121, 249, 0.15);
        border-radius: 1rem;
        color: var(--sb-text-dim);
        font-size: 0.7rem;
        cursor: pointer;
        transition: all 0.2s;
    }
    
    .filter-btn:hover {
        background: rgba(232, 121, 249, 0.15);
        border-color: rgba(232, 121, 249, 0.3);
        color: var(--sb-text-main);
    }
    
    .filter-btn.active {
        background: rgba(232, 121, 249, 0.25);
        border-color: rgba(232, 121, 249, 0.5);
        color: var(--sb-text-main);
    }
    
    .filter-btn .persona-avatar {
        font-size: 0.8rem;
    }
    
    /* Persona Groups */
    .persona-group {
        margin-bottom: 1rem;
    }
    
    .persona-group-header {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 0.25rem;
        margin-bottom: 0.5rem;
        border-bottom: 1px solid rgba(232, 121, 249, 0.15);
    }
    
    .persona-group-header .persona-avatar {
        font-size: 1rem;
    }
    
    .persona-group-name {
        flex: 1;
        font-size: 0.75rem;
        font-weight: 600;
        color: var(--sb-accent);
    }
    
    .group-count {
        padding: 0.125rem 0.375rem;
        background: rgba(232, 121, 249, 0.15);
        border-radius: 0.75rem;
        font-size: 0.65rem;
        color: var(--sb-text-dim);
    }
    
    .group-items {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding-left: 0.5rem;
    }
    
    /* Meta rows for dreams/journal */
    .dream-meta, .journal-meta {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: 0.5rem;
    }
    
    .item-tags {
        display: flex;
        gap: 0.25rem;
    }
    
    .tag-dot {
        width: 6px;
        height: 6px;
        border-radius: 50%;
    }
</style>