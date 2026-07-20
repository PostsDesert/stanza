import { Component, For, Show, onMount, createSignal, createMemo } from 'solid-js';
import { useNavigate, useSearchParams } from '@solidjs/router';
import { messagesStore, fetchMessages, addMessage, updateMessage, deleteMessage, initOfflineMessages, syncOutbox } from '../stores/messagesStore';
import { showToast, uiStore } from '../stores/uiStore';
import { MessageCard } from '../components/MessageCard';
import { MessageInput } from '../components/MessageInput';
import { LoadingSpinner } from '../components/LoadingSpinner';
import { EditModal } from '../components/EditModal';
import { SearchBar } from '../components/SearchBar';
import { HeaderMenu } from '../components/HeaderMenu';
import { SyncStatus } from '../components/SyncStatus';
import { api } from '../services/api';
import { parseSearchQuery } from '../utils/search';
import type { Message, SearchQuery } from '../types';
import './Feed.css';

export const Feed: Component = () => {
    const navigate = useNavigate();
    const [searchParams, setSearchParams] = useSearchParams();
    const [isLoading, setIsLoading] = createSignal(true);
    const [selectedMessage, setSelectedMessage] = createSignal<string | null>(null);
    const [editingMessageId, setEditingMessageId] = createSignal<string | null>(null);
    const [isEditSaving, setIsEditSaving] = createSignal(false);

    // Search state
    const [isSearching, setIsSearching] = createSignal(false);
    const [isSearchActive, setIsSearchActive] = createSignal(false);
    const [searchResults, setSearchResults] = createSignal<Message[]>([]);
    const [searchTotal, setSearchTotal] = createSignal(0);
    const [searchQuery, setSearchQuery] = createSignal('');

    // Determine which messages to display
    const displayMessages = createMemo(() => {
        if (isSearchActive()) {
            return searchResults();
        }
        return messagesStore.messages;
    });

    // Get the content of the message being edited
    const editingMessageContent = createMemo(() => {
        const id = editingMessageId();
        if (!id) return '';
        const message = messagesStore.messages.find(m => m.id === id);
        return message?.content || '';
    });

    const handleSubmit = async (content: string) => {
        try {
            const message = await addMessage(content);
            if (message.syncState === 'pending') {
                showToast('Saved offline, will sync automatically', 'info');
            } else {
                showToast('Message posted!', 'success');
            }
        } catch (err) {
            showToast('Failed to post message', 'error');
        }
    };

    const handleDelete = async (id: string) => {
        if (!confirm('Delete this message?')) return;

        try {
            await deleteMessage(id);
            if (uiStore.isOnline) {
                showToast('Message deleted', 'info');
            } else {
                showToast('Delete queued for sync', 'info');
            }
        } catch (err) {
            showToast('Failed to delete message', 'error');
        }
    };

    const handleEditSave = async (content: string) => {
        const id = editingMessageId();
        if (!id) return;

        setIsEditSaving(true);
        try {
            const updated = await updateMessage(id, content);
            if (updated.syncState === 'pending') {
                showToast('Edit saved offline, will sync automatically', 'info');
            } else {
                showToast('Message updated!', 'success');
            }
            setEditingMessageId(null);
        } catch (err) {
            showToast('Failed to update message', 'error');
        } finally {
            setIsEditSaving(false);
        }
    };

    const handleSearch = async (query: SearchQuery) => {
        setSearchParams({ q: searchQuery() });
        setIsSearching(true);
        try {
            const response = await api.searchMessages(query);
            setSearchResults(response.messages);
            setSearchTotal(response.total);
            setIsSearchActive(true);
        } catch (err) {
            showToast('Search failed', 'error');
        } finally {
            setIsSearching(false);
        }
    };

    const handleTagClick = (tag: string) => {
        setSearchQuery(`tag:${tag}`);
        handleSearch({ tags: tag });
    };

    const handleClearSearch = () => {
        setSearchParams({ q: undefined });
        setIsSearchActive(false);
        setSearchResults([]);
        setSearchTotal(0);
        setSearchQuery('');
    };

    const handleLogoClick = () => {
        handleClearSearch();
        navigate('/');
    };

    onMount(async () => {
        await initOfflineMessages();

        // Check for search query in URL
        const q = searchParams.q;
        if (q) {
            setSearchQuery(q);
            const query = parseSearchQuery(q);
            // Don't await search here to allow fetching messages in parallel
            handleSearch(query);
        }

        try {
            if (uiStore.isOnline) {
                await fetchMessages();
                await syncOutbox();
            }
        } catch (err) {
            if (messagesStore.messages.length === 0) {
                showToast('Failed to load messages', 'error');
            }
        } finally {
            setIsLoading(false);
        }
    });

    return (
        <div class="feed-page">
            <header class="feed-header">
                <h1 class="feed-title" onClick={handleLogoClick}>Stanza</h1>
                <SearchBar
                    query={searchQuery()}
                    onQueryChange={setSearchQuery}
                    onSearch={handleSearch}
                    onClear={handleClearSearch}
                    isSearching={isSearching()}
                    isSearchActive={isSearchActive()}
                />
                <SyncStatus compactOnMobile />
                <HeaderMenu />
            </header>

            <main class="feed-main">
                <Show when={isLoading()}>
                    <div class="feed-loading">
                        <LoadingSpinner size="lg" />
                    </div>
                </Show>

                <Show when={isSearchActive() && !isSearching()}>
                    <div class="search-results-info">
                        <span>{searchTotal()} result{searchTotal() !== 1 ? 's' : ''} found</span>
                    </div>
                </Show>

                <Show when={!isLoading() && displayMessages().length === 0 && !isSearchActive()}>
                    <div class="feed-empty">
                        <p>No messages yet.</p>
                        <p>Start typing below!</p>
                    </div>
                </Show>

                <Show when={!isLoading() && isSearchActive() && displayMessages().length === 0}>
                    <div class="feed-empty">
                        <p>No results found.</p>
                        <p>Try a different search term.</p>
                    </div>
                </Show>

                <Show when={!isLoading() && displayMessages().length > 0}>
                    <div class="message-list">
                        <For each={displayMessages()}>
                            {(message) => (
                                <MessageCard
                                    message={message}
                                    onClick={() => navigate(`/post/${message.id}`)}
                                    onEdit={() => setEditingMessageId(message.id)}
                                    onDelete={() => handleDelete(message.id)}
                                    onTagClick={handleTagClick}
                                />
                            )}
                        </For>
                    </div>
                </Show>
            </main>

            <MessageInput onSubmit={handleSubmit} />

            <EditModal
                isOpen={editingMessageId() !== null}
                initialContent={editingMessageContent()}
                onSave={handleEditSave}
                onClose={() => setEditingMessageId(null)}
                isLoading={isEditSaving()}
            />
        </div>
    );
};

export default Feed;
