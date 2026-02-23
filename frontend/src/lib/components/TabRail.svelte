<script lang="ts">
    import { appState } from '$lib/state.svelte';
    import { tick } from 'svelte';
    import ColorPicker from './ColorPicker.svelte';
    
    // Local state for renaming
    let editingChatId = $state<string | null>(null);
    let editInputRef = $state<HTMLInputElement | null>(null);

    // --- Handlers ---

    function handleDragStart(e: DragEvent, chatId: string) {
        appState.draggedChatId = chatId;
        if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
    }

    function handleDragOver(e: DragEvent) {
        e.preventDefault();
        e.dataTransfer!.dropEffect = 'move';
    }

    function handleDropOnTab(e: DragEvent, targetGroupId: string, targetIndex: number) {
        e.preventDefault();
        e.stopPropagation();
        if (appState.draggedChatId) {
            appState.moveTab(appState.draggedChatId, targetGroupId, targetIndex);
            appState.draggedChatId = null;
        }
    }

    function handleDropOnGroup(e: DragEvent, targetGroupId: string) {
        e.preventDefault();
        if (appState.draggedChatId) {
            appState.moveTab(appState.draggedChatId, targetGroupId);
            appState.draggedChatId = null;
        }
    }

    // --- Renaming Logic ---

    async function startEditing(chatId: string) {
        editingChatId = chatId;
        // Wait for DOM to render the input, then focus it
        await tick(); 
        if (editInputRef) {
            editInputRef.focus();
            editInputRef.select();
        }
    }

    function finishEditing(chatId: string, newTitle: string, save: boolean) {
        if (save) {
            appState.renameChat(chatId, newTitle);
        }
        editingChatId = null;
    }

    function handleKeyDown(e: KeyboardEvent, chatId: string, currentTitle: string) {
        if (e.key === 'Enter') {
            finishEditing(chatId, currentTitle, true);
        } else if (e.key === 'Escape') {
            finishEditing(chatId, currentTitle, false); // Cancel
        }
    }
</script>

<div class="flex flex-col gap-2 p-2 bg-zinc-950 border-b border-zinc-800 select-none">
    <div class="flex flex-wrap gap-4 items-start">
        
        {#each appState.groups as group (group.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div 
                class="flex flex-col rounded-t-lg border-t-2 bg-zinc-900/50 transition-all duration-200"
                style="border-color: {group.color}"
                ondragover={handleDragOver}
                ondrop={(e) => handleDropOnGroup(e, group.id)}
            >
                <div class="flex items-center justify-between px-3 py-1.5 bg-zinc-900 rounded-t-lg cursor-grab active:cursor-grabbing hover:bg-zinc-800 transition-colors min-w-[150px]">
                    <div class="flex items-center gap-2">
                        <ColorPicker bind:value={group.color} />
                        
                        {#if !group.collapsed}
                            <input 
                                bind:value={group.name} 
                                class="bg-transparent text-xs font-bold text-zinc-400 focus:outline-none focus:text-white w-24 truncate" 
                            />
                        {:else}
                             <span class="text-xs font-bold text-zinc-400">{group.name}</span>
                        {/if}
                    </div>
                    
                    <div class="flex items-center">
                        <button onclick={() => group.collapsed = !group.collapsed} class="text-zinc-500 hover:text-white ml-2 px-1">
                            {group.collapsed ? '+' : '-'}
                        </button>
                        <button onclick={() => appState.addChat(group.id)} class="text-zinc-500 hover:text-green-400 ml-1 text-xs px-1" title="New Chat">
                            +
                        </button>
                    </div>
                </div>

                {#if !group.collapsed}
                    <div class="flex items-center p-1 gap-1 overflow-x-auto max-w-xl scrollbar-hide">
                        {#each group.chatIds as chatId, index (chatId)}
                            {@const chat = appState.chats[chatId]}
                            
                            <div 
                                draggable="true"
                                ondragstart={(e) => handleDragStart(e, chatId)}
                                ondrop={(e) => handleDropOnTab(e, group.id, index)}
                                onclick={() => appState.activeChatId = chat.id}
                                onkeypress={() => appState.activeChatId = chat.id}
                                ondblclick={() => startEditing(chat.id)}
                                role="tab"
                                tabindex="0"
                                class={`
                                    group/tab relative flex items-center gap-2 px-3 py-1.5 rounded-md text-sm border cursor-pointer transition-all min-w-[100px] max-w-[160px]
                                    ${appState.activeChatId === chat.id 
                                        ? 'bg-zinc-800 text-white border-zinc-700 shadow-sm' 
                                        : 'border-transparent text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'}
                                    ${appState.draggedChatId === chat.id ? 'opacity-40 border-dashed border-zinc-600' : ''}
                                `}
                            >
                                {#if editingChatId === chat.id}
                                    <input 
                                        bind:this={editInputRef}
                                        value={chat.title}
                                        onblur={(e) => finishEditing(chat.id, e.currentTarget.value, true)}
                                        onkeydown={(e) => handleKeyDown(e, chat.id, e.currentTarget.value)}
                                        onclick={(e) => e.stopPropagation()} 
                                        class="bg-zinc-950 text-white px-1 py-0.5 rounded outline-none border border-blue-500 w-full text-xs"
                                    />
                                {:else}
                                    <span class="truncate text-xs pointer-events-none select-none">{chat.title}</span>
                                {/if}
                                
                                {#if editingChatId !== chat.id}
                                    <button 
                                        onclick={(e) => { e.stopPropagation(); appState.closeChat(chat.id); }}
                                        class="opacity-0 group-hover/tab:opacity-100 hover:bg-red-500/20 hover:text-red-400 rounded w-4 h-4 flex items-center justify-center text-[10px] absolute right-1"
                                    >✕</button>
                                {/if}
                            </div>
                        {/each}
                    </div>
                {/if}
            </div>
        {/each}
        
        <button 
            onclick={() => {
                const newId = crypto.randomUUID();
                appState.groups.push({ id: newId, name: 'New Group', color: '#71717a', collapsed: false, chatIds: [] });
            }}
            class="px-3 py-2 text-zinc-500 hover:text-zinc-300 border border-dashed border-zinc-800 hover:border-zinc-600 rounded-lg text-xs transition-colors"
        >
            + New Group
        </button>
    </div>
</div>