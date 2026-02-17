import type { Component } from 'solid-js';
import { messagesStore } from '../stores/messagesStore';
import { uiStore } from '../stores/uiStore';
import './SyncStatus.css';

export const SyncStatus: Component = () => {
    const statusClass = () => {
        if (!uiStore.isOnline) return 'is-offline';
        if (messagesStore.failedCount > 0) return 'is-failed';
        if (messagesStore.isReplaying) return 'is-syncing';
        if (messagesStore.pendingCount > 0) return 'is-pending';
        return 'is-synced';
    };

    const statusLabel = () => {
        if (!uiStore.isOnline) return 'Offline';
        if (messagesStore.authRequiredForReplay) return 'Login required to sync';
        if (messagesStore.failedCount > 0) return `${messagesStore.failedCount} failed`;
        if (messagesStore.isReplaying) return 'Syncing...';
        if (messagesStore.pendingCount > 0) return `${messagesStore.pendingCount} queued`;
        return 'Synced';
    };

    return (
        <div class={`sync-status ${statusClass()}`} role="status" aria-live="polite">
            <span class="sync-status-dot" />
            <span class="sync-status-label">{statusLabel()}</span>
        </div>
    );
};
