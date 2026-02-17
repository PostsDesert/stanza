import type { Component } from 'solid-js';
import { messagesStore } from '../stores/messagesStore';
import { uiStore } from '../stores/uiStore';
import './SyncStatus.css';

type SyncStatusProps = {
    variant?: 'pill' | 'menu';
    compactOnMobile?: boolean;
};

export const SyncStatus: Component<SyncStatusProps> = (props) => {
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
        <div
            class={`sync-status ${statusClass()} ${props.variant === 'menu' ? 'is-menu' : ''} ${props.compactOnMobile ? 'is-compact-mobile' : ''}`}
            role="status"
            aria-live="polite"
            aria-label={`Sync status: ${statusLabel()}`}
        >
            <span class="sync-status-dot" />
            <span class="sync-status-label">{statusLabel()}</span>
        </div>
    );
};
