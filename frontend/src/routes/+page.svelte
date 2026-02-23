<script lang="ts">
    import ChatMessage from '$lib/components/ChatMessage.svelte';
    import ChatInput from '$lib/components/ChatInput.svelte';
    import Sidebar from '$lib/components/Sidebar.svelte';
    import PersonaEditor from '$lib/components/PersonaEditor.svelte';
    import JournalViewer from '$lib/components/JournalViewer.svelte';
    import DreamViewer from '$lib/components/DreamViewer.svelte';
    import MessageEditor from '$lib/components/MessageEditor.svelte';
    import ProfileViewer from '$lib/components/ProfileViewer.svelte';
    import ThinkingIndicator from '$lib/components/ThinkingIndicator.svelte';
    import { appState, type Persona } from '$lib/store.svelte';
    import { onMount, tick } from 'svelte';

    let scrollContainer: HTMLElement | undefined = $state(undefined);
    let bottomAnchor: HTMLDivElement | undefined = $state(undefined);
    let isNearBottom = $state(true);
    let showScrollButton = $state(false);
    let lastMessageCount = $state(0);
    
    // Profile viewer state
    let viewingPersona = $state<Persona | null>(null);
    
    // Derived chat info for header
    let chatTitle = $derived(appState.currentChat?.title || 'New Chat');
    let branchName = $derived(appState.currentBranch()?.name || 'Main');
    
    // Title editing state
    let isEditingTitle = $state(false);
    let editingTitle = $state('');
    let titleInputRef: HTMLInputElement | undefined = $state(undefined);
    
    function startEditingTitle() {
        editingTitle = chatTitle;
        isEditingTitle = true;
        tick().then(() => {
            titleInputRef?.focus();
            titleInputRef?.select();
        });
    }
    
    function saveTitle() {
        const trimmed = editingTitle.trim();
        if (trimmed && trimmed !== chatTitle && appState.currentChatId) {
            appState.renameChat(appState.currentChatId, trimmed);
        }
        isEditingTitle = false;
    }
    
    function cancelEditingTitle() {
        isEditingTitle = false;
        editingTitle = '';
    }
    
    function handleTitleKeydown(e: KeyboardEvent) {
        if (e.key === 'Enter') {
            e.preventDefault();
            saveTitle();
        } else if (e.key === 'Escape') {
            e.preventDefault();
            cancelEditingTitle();
        }
    }
    
    // Find last user message index
    let lastUserMessageIndex = $derived(() => {
        const messages = appState.messages;
        for (let i = messages.length - 1; i >= 0; i--) {
            if (messages[i].role === 'user') {
                return i;
            }
        }
        return -1;
    });

    function scrollToBottom() {
        if (!scrollContainer) return;

        // Smooth-scroll to the bottom
        scrollContainer.scrollTo({
            top: scrollContainer.scrollHeight,
            behavior: 'smooth'
        });
        // Re-check position after animation
        setTimeout(checkScrollPosition, 350);
    }

    function scrollToBottomInstant() {
        if (!scrollContainer) return;
        scrollContainer.scrollTop = scrollContainer.scrollHeight;
        checkScrollPosition();
    }

    function checkScrollPosition() {
        if (!scrollContainer) return;
        const { scrollTop, scrollHeight, clientHeight } = scrollContainer;
        
        // Calculate if we're near the bottom (within 80px)
        const distanceFromBottom = scrollHeight - scrollTop - clientHeight;
        const atBottom = distanceFromBottom <= 80;

        isNearBottom = atBottom;
        // Show button when we have enough content to scroll AND we're not at bottom
        const hasScrollableContent = scrollHeight > clientHeight + 50;
        showScrollButton = hasScrollableContent && !atBottom;
    }

    // Auto-scroll when new messages arrive (if user is near bottom)
    $effect(() => {
        const messageCount = appState.messages.length;
        // Only scroll when new messages are added, not on initial load
        if (messageCount > lastMessageCount && lastMessageCount > 0 && isNearBottom) {
            tick().then(() => {
                scrollToBottom();
            });
        }
        lastMessageCount = messageCount;
    });

    // Setup resize observer and initial scroll when scroll container is available
    $effect(() => {
        if (!scrollContainer) return;
        
        const resizeObserver = new ResizeObserver(() => {
            checkScrollPosition();
        });
        resizeObserver.observe(scrollContainer);
        
        // Add scroll listener directly for reliability
        const handleScroll = () => checkScrollPosition();
        scrollContainer.addEventListener('scroll', handleScroll, { passive: true });
        
        // Initial scroll to bottom when container becomes available
        tick().then(() => {
            scrollToBottomInstant();
            // Double-check after a brief delay for layout settling
            setTimeout(() => {
                scrollToBottomInstant();
            }, 50);
        });
        
        return () => {
            resizeObserver.disconnect();
            scrollContainer?.removeEventListener('scroll', handleScroll);
        };
    });

    // Watch for scroll to bottom triggers (e.g., when switching chats)
    $effect(() => {
        const signal = appState.scrollToBottomSignal;
        if (signal > 0 && scrollContainer) {
            tick().then(() => {
                scrollToBottomInstant();
                // Double-check after layout settles
                setTimeout(() => {
                    scrollToBottomInstant();
                }, 50);
            });
        }
    });
    
    // Also watch for chat changes directly (only after initialization)
    $effect(() => {
        const chatId = appState.currentChatId;
        const initialized = appState.isInitialized;
        if (initialized && chatId && scrollContainer) {
            // Use requestAnimationFrame to ensure DOM has updated
            requestAnimationFrame(() => {
                scrollToBottomInstant();
                setTimeout(() => {
                    scrollToBottomInstant();
                }, 100);
            });
        }
    });

    onMount(() => {
        // Initialize message count to prevent auto-scroll on initial load
        lastMessageCount = appState.messages.length;
        
        // Additional delayed scroll checks for edge cases with async data loading
        setTimeout(() => {
            scrollToBottomInstant();
            checkScrollPosition();
        }, 100);
        setTimeout(() => {
            scrollToBottomInstant();
            checkScrollPosition();
        }, 300);
        setTimeout(() => {
            scrollToBottomInstant();
            checkScrollPosition();
        }, 500);
        // Also check after a longer delay for slow loading
        setTimeout(() => {
            scrollToBottomInstant();
            checkScrollPosition();
        }, 1000);
    });
</script>

<div class="flex h-screen bg-midnight-950">
    <!-- Sidebar -->
    <Sidebar />
    
    <!-- Main chat area - offset by sidebar when open -->
    <div class="flex-1 flex flex-col transition-all" style={appState.isSidebarOpen ? 'margin-left: 18rem;' : 'margin-left: 3.5rem;'}>
        <!-- Header - fixed at top, shows chat title and path -->
        <header class="fixed top-0 bg-midnight-900/80 backdrop-blur border-b border-midnight-700/50 py-3 flex items-center justify-center z-40 transition-all" style={appState.isSidebarOpen ? 'left: 18rem; right: 0; display: flex; justify-content: center;' : 'left: 3.5rem; right: 0; display: flex; justify-content: center;'}>
            <div class="">
                {#if isEditingTitle}
                    <input
                        bind:this={titleInputRef}
                        bind:value={editingTitle}
                        onblur={saveTitle}
                        onkeydown={handleTitleKeydown}
                        class="title-edit-input"
                    />
                {:else}
                    <h1 
                        class="text-lg font-medium text-midnight-100 cursor-pointer hover:text-accent-magenta transition-colors"
                        ondblclick={startEditingTitle}
                        title="Double-click to rename"
                    >✦ {chatTitle} ✦</h1>
                {/if}
                <h3 class="text-midnight-400 text-xs tracking-wide" style="justify-content: center; display: flex;">
                    <i>- {branchName} -</i>
                </h3>
            </div>
        </header>
        
        <!-- Messages container - with padding for fixed header and input -->
        <div bind:this={scrollContainer} onscroll={checkScrollPosition} class="flex-1 overflow-y-auto px-6 py-8 space-y-4" style="padding-bottom: 16rem; overflow-x: hidden; margin-top: 3.5rem; margin-left: 1rem;">
            {#if appState.messages.length === 0}
                <div class="h-full flex items-center justify-center">
                    <div class="text-center">
                        <div class="text-6xl mb-4 animate-float">✨</div>
                        <h2 class="text-4xl font-bold text-midnight-100 mb-3">Welcome~</h2>
                        <p class="text-midnight-400">Let's begin our conversation. I'm listening...</p>
                    </div>
                </div>
            {:else}
                {#each appState.messages as message, idx (message.id || message.timestamp || `msg-${idx}`)}
                    <ChatMessage {message} isLastUserMessage={idx === lastUserMessageIndex()} onviewprofile={(p: Persona) => viewingPersona = p} />
                {/each}
                
                <!-- Thinking indicator - shows while AI is processing -->
                <ThinkingIndicator />
            {/if}

        <div bind:this={bottomAnchor} aria-hidden="true"></div>
        </div>
        
        <!-- Scroll to bottom button - always rendered, visibility controlled by CSS -->
        <button
            class="scroll-to-bottom-btn"
            class:visible={showScrollButton}
            onclick={scrollToBottom}
            title="Jump to latest messages"
            style={appState.isSidebarOpen ? 'left: calc(50% + 9rem);' : 'left: calc(50% + 1.75rem);'}
        >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M12 5v14M5 12l7 7 7-7"/>
            </svg>
        </button>

        <!-- Input - ChatInput has its own fixed positioning -->
        <ChatInput />
    </div>
</div>

<!-- Persona Editor Overlay -->
<PersonaEditor />

<!-- Journal Viewer Overlay -->
<JournalViewer />

<!-- Dream Viewer Overlay -->
<DreamViewer />

<!-- Message Editor Overlay -->
<MessageEditor />

<!-- Profile Viewer Overlay -->
{#if viewingPersona}
    <ProfileViewer persona={viewingPersona} onclose={() => viewingPersona = null} />
{/if}

<style>
    :global(.glow) {
        animation: glow 3s ease-in-out infinite;
    }
    
    @keyframes glow {
        0%, 100% {
            text-shadow: 0 0 10px rgba(232, 121, 249, 0.3), 0 0 20px rgba(232, 121, 249, 0.2);
        }
        50% {
            text-shadow: 0 0 20px rgba(232, 121, 249, 0.6), 0 0 40px rgba(232, 121, 249, 0.4);
        }
    }

    :global(.animate-float) {
        animation: float 4s ease-in-out infinite;
    }

    @keyframes float {
        0%, 100% {
            transform: translateY(0px);
            opacity: 1;
        }
        50% {
            transform: translateY(-30px);
            opacity: 0.7;
        }
    }

    .scroll-to-bottom-btn {
        position: fixed;
        bottom: 16rem;
        left: 50%;
        transform: translateX(-50%) translateY(10px);
        z-index: 60;
        width: 2.5rem;
        height: 2.5rem;
        padding: 0.5rem;
        border-radius: 50%;
        background: rgba(232, 121, 249, 0.2);
        border: 1px solid rgba(232, 121, 249, 0.4);
        color: rgba(232, 121, 249, 0.9);
        cursor: pointer;
        display: flex;
        align-items: center;
        justify-content: center;
        transition: all 0.25s ease-out;
        box-shadow: 0 4px 15px rgba(232, 121, 249, 0.2);
        opacity: 0;
        pointer-events: none;
    }
    
    .scroll-to-bottom-btn.visible {
        opacity: 1;
        pointer-events: auto;
        transform: translateX(-50%) translateY(0);
    }

    .scroll-to-bottom-btn:hover {
        background: rgba(232, 121, 249, 0.3);
        color: rgba(232, 121, 249, 1);
        border-color: rgba(232, 121, 249, 0.6);
        box-shadow: 0 4px 20px rgba(232, 121, 249, 0.4);
    }

    .scroll-to-bottom-btn svg {
        width: 1.25rem;
        height: 1.25rem;
    }

    .title-edit-input {
        font-size: 1.125rem;
        font-weight: 500;
        color: #e8eaec;
        background-color: #121314;
        border: 1px solid #35393a;
        border-radius: 0.5rem;
        padding: 0.375rem 1rem;
        text-align: center;
        outline: none;
        min-width: 220px;
        caret-color: #e879f9;
        box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.3);
    }

    .title-edit-input:focus {
        border-color: #e879f9;
        box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.3), 0 0 0 2px rgba(232, 121, 249, 0.2);
    }
</style>