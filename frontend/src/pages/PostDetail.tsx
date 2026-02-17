import { Component, Show, createMemo, onMount, createSignal } from 'solid-js';
import { useParams, useNavigate } from '@solidjs/router';
import { messagesStore, fetchMessages, deleteMessage, updateMessage, initOfflineMessages, syncOutbox } from '../stores/messagesStore';
import { formatDate, formatRelativeTime, isWithinMinutes } from '../utils/date';
import { LoadingSpinner } from '../components/LoadingSpinner';
import { EditModal } from '../components/EditModal';
import { EditIcon } from '../components/icons/EditIcon';
import { DeleteIcon } from '../components/icons/DeleteIcon';
import { showToast, uiStore } from '../stores/uiStore';
import { SyncStatus } from '../components/SyncStatus';
import './PostDetail.css';

const PostDetail: Component = () => {
    const params = useParams();
    const navigate = useNavigate();
    const [isEditSaving, setIsEditSaving] = createSignal(false);
    const [isEditing, setIsEditing] = createSignal(false);

    // Try to find the message in the store
    const message = createMemo(() =>
        messagesStore.messages.find(m => m.id === params.id)
    );

    // Timestamp logic
    const timestamp = createMemo(() => {
        const msg = message();
        if (!msg) return '';
        if (isWithinMinutes(msg.created_at, 24 * 60)) {
            return formatRelativeTime(msg.created_at);
        }
        return formatDate(msg.created_at);
    });

    const syncBadge = createMemo(() => {
        const msg = message();
        if (!msg) return null;
        if (msg.syncState === 'failed') {
            return { label: 'Sync failed', className: 'post-sync-badge is-failed' };
        }
        if (msg.syncState === 'pending') {
            return { label: 'Queued', className: 'post-sync-badge is-pending' };
        }
        return null;
    });

    // If message not found, try fetching (in case of direct link or refresh)
    onMount(async () => {
        await initOfflineMessages();

        if (!message() && messagesStore.messages.length === 0) {
            try {
                if (uiStore.isOnline) {
                    await fetchMessages();
                    await syncOutbox();
                }
            } catch (error) {
                console.error('Failed to load messages', error);
            }
        }
    });

    const handleBack = () => {
        navigate('/');
    };

    const handleDelete = async () => {
        const msg = message();
        if (!msg) return;
        if (!confirm('Delete this message?')) return;

        try {
            await deleteMessage(msg.id);
            if (uiStore.isOnline) {
                showToast('Message deleted', 'info');
            } else {
                showToast('Delete queued for sync', 'info');
            }
            navigate('/');
        } catch (err) {
            showToast('Failed to delete message', 'error');
        }
    };

    const handleEditSave = async (content: string) => {
        const msg = message();
        if (!msg) return;

        setIsEditSaving(true);
        try {
            const updated = await updateMessage(msg.id, content);
            if (updated.syncState === 'pending') {
                showToast('Edit saved offline, will sync automatically', 'info');
            } else {
                showToast('Message updated!', 'success');
            }
            setIsEditing(false);
        } catch (err) {
            showToast('Failed to update message', 'error');
        } finally {
            setIsEditSaving(false);
        }
    };

    const handleTagClick = (tag: string) => {
        navigate(`/?q=tag:${tag}`);
    };

    const formatContent = (text: string) => {
        const parts = text.split(/(#\w+)/g);
        return parts.map((part) => {
            if (part.startsWith('#') && part.length > 1) {
                const tag = part.substring(1);
                return (
                    <span
                        class="hashtag"
                        onClick={(e) => {
                            e.stopPropagation();
                            handleTagClick(tag);
                        }}
                    >
                        {part}
                    </span>
                );
            }
            return part;
        });
    };

    return (
        <div class="post-detail-page">
            <header class="post-detail-header">
                <button
                    class="back-button"
                    onClick={handleBack}
                    aria-label="Back to Feed"
                >
                    ← Back
                </button>
                <SyncStatus />
                <Show when={message()}>
                    <div class="post-actions">
                        <button
                            class="action-button edit-button"
                            onClick={() => setIsEditing(true)}
                            aria-label="Edit message"
                            title="Edit"
                        >
                            <EditIcon width="16" height="16" /> Edit
                        </button>
                        <button
                            class="action-button delete-button"
                            onClick={handleDelete}
                            aria-label="Delete message"
                            title="Delete"
                        >
                            <DeleteIcon width="16" height="16" /> Delete
                        </button>
                    </div>
                </Show>
            </header>

            <main class="post-detail-main">
                <div class="post-meta">
                    <span class="post-timestamp">{timestamp()}</span>
                    <Show when={syncBadge()}>
                        {(badge) => (
                            <span class={badge().className} aria-label={badge().label}>
                                {badge().label}
                            </span>
                        )}
                    </Show>
                </div>

                <Show when={messagesStore.isSyncing && !message()}>
                    <div class="loading-container">
                        <LoadingSpinner size="lg" />
                    </div>
                </Show>

                <Show when={!messagesStore.isSyncing && !message()}>
                    <div class="error-container">
                        <p>Message not found.</p>
                        <button onClick={handleBack} class="secondary-button">Return to Feed</button>
                    </div>
                </Show>

                <Show when={message()}>
                    {(msg) => (
                        <>
                            <article class="full-post">
                                <div class="post-content">
                                    {formatContent(msg().content)}
                                </div>
                            </article>

                            <EditModal
                                isOpen={isEditing()}
                                initialContent={msg().content}
                                onSave={handleEditSave}
                                onClose={() => setIsEditing(false)}
                                isLoading={isEditSaving()}
                            />
                        </>
                    )}
                </Show>
            </main>
        </div>
    );
};

export default PostDetail;
