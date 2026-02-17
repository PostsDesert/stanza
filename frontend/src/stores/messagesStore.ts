import { createSignal } from 'solid-js';
import { api, ApiError } from '../services/api';
import {
    loadCachedMessages,
    loadOutbox,
    saveCachedMessages,
    saveOutbox,
    purgeOfflineDataForUser,
} from '../services/offlineStore';
import type { Message, PendingOperation } from '../types';

const CURRENT_USER_ID_KEY = 'current_user_id';

// Messages state
const [messages, setMessages] = createSignal<Message[]>([]);
const [isSyncing, setIsSyncing] = createSignal<boolean>(false);
const [lastSync, setLastSync] = createSignal<string | null>(null);

// Offline queue and replay state
const [outbox, setOutbox] = createSignal<PendingOperation[]>([]);
const [isReplaying, setIsReplaying] = createSignal<boolean>(false);
const [lastReplayAt, setLastReplayAt] = createSignal<string | null>(null);
const [lastReplayError, setLastReplayError] = createSignal<string | null>(null);
const [authRequiredForReplay, setAuthRequiredForReplay] = createSignal<boolean>(false);

let stopAutoSyncHandler: (() => void) | null = null;

// Store object for reactive access
export const messagesStore = {
    get messages() { return messages(); },
    get isSyncing() { return isSyncing(); },
    get lastSync() { return lastSync(); },
    get isReplaying() { return isReplaying(); },
    get lastReplayAt() { return lastReplayAt(); },
    get lastReplayError() { return lastReplayError(); },
    get authRequiredForReplay() { return authRequiredForReplay(); },
    get pendingCount() { return outbox().filter((op) => op.status !== 'failed').length; },
    get failedCount() { return outbox().filter((op) => op.status === 'failed').length; },
    get failedOperations() { return outbox().filter((op) => op.status === 'failed'); },
};

function getCurrentUserId(): string | null {
    if (typeof window === 'undefined') return null;
    return localStorage.getItem(CURRENT_USER_ID_KEY);
}

function getNowIso(): string {
    return new Date().toISOString();
}

function sortMessages(msgs: Message[]): Message[] {
    return [...msgs].sort((a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );
}

function isOnline(): boolean {
    return typeof navigator === 'undefined' ? true : navigator.onLine;
}

function isNetworkError(error: unknown): boolean {
    return error instanceof TypeError;
}

function createOperationId(): string {
    return typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
        ? crypto.randomUUID()
        : `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function createMessageId(): string {
    return typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
        ? crypto.randomUUID()
        : `local-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

async function persistMessages(): Promise<void> {
    const userId = getCurrentUserId();
    if (!userId) return;
    await saveCachedMessages(userId, messages());
}

async function persistOutbox(): Promise<void> {
    const userId = getCurrentUserId();
    if (!userId) return;
    await saveOutbox(userId, outbox());
}

function mergeSyncedIntoMessages(incoming: Message[]): void {
    setMessages((prev) => {
        const merged = [...prev];
        for (const msg of incoming) {
            const normalized: Message = {
                ...msg,
                syncState: 'synced',
                localUpdatedAt: msg.updated_at,
            };
            const idx = merged.findIndex((m) => m.id === msg.id);
            if (idx >= 0) {
                merged[idx] = normalized;
            } else {
                merged.push(normalized);
            }
        }
        return sortMessages(merged);
    });
}

function setMessageSyncState(messageId: string, syncState: Message['syncState']): void {
    setMessages((prev) => prev.map((m) => (
        m.id === messageId
            ? { ...m, syncState, localUpdatedAt: getNowIso() }
            : m
    )));
}

function applyOutboxCompaction(existing: PendingOperation[], nextOp: PendingOperation): PendingOperation[] {
    const result = [...existing];
    const messageOps = result
        .filter((op) => op.messageId === nextOp.messageId)
        .sort((a, b) => new Date(a.updatedAt).getTime() - new Date(b.updatedAt).getTime());

    if (messageOps.length === 0) {
        result.push(nextOp);
        return result;
    }

    const latest = messageOps[messageOps.length - 1];

    // Keep explicit failed operations for user resolution.
    if (latest.status === 'failed') {
        result.push(nextOp);
        return result;
    }

    if (latest.type === 'create' && nextOp.type === 'update') {
        latest.payload = { content: (nextOp.payload as { content: string }).content };
        latest.updatedAt = nextOp.updatedAt;
        return result;
    }

    if (latest.type === 'create' && nextOp.type === 'delete') {
        return result.filter((op) => op.messageId !== nextOp.messageId || op.status === 'failed');
    }

    if (latest.type === 'update' && nextOp.type === 'update') {
        latest.payload = { content: (nextOp.payload as { content: string }).content };
        latest.updatedAt = nextOp.updatedAt;
        return result;
    }

    if (latest.type === 'update' && nextOp.type === 'delete') {
        latest.type = 'delete';
        latest.payload = { content: '' };
        latest.updatedAt = nextOp.updatedAt;
        return result;
    }

    if (latest.type === 'delete' && nextOp.type === 'update') {
        result.push(nextOp);
        return result;
    }

    if (latest.type === 'delete' && nextOp.type === 'delete') {
        return result;
    }

    result.push(nextOp);
    return result;
}

async function enqueueOperation(op: PendingOperation): Promise<void> {
    setOutbox((prev) => applyOutboxCompaction(prev, op));
    await persistOutbox();
}

function markOperationStatus(
    opId: string,
    updates: Partial<Pick<PendingOperation, 'status' | 'attempts' | 'lastError' | 'lastHttpStatus' | 'updatedAt'>>,
): void {
    setOutbox((prev) => prev.map((op) => (
        op.opId === opId
            ? { ...op, ...updates }
            : op
    )));
}

function removeOperation(opId: string): void {
    setOutbox((prev) => prev.filter((op) => op.opId !== opId));
}

export async function initOfflineMessages(): Promise<void> {
    const userId = getCurrentUserId();
    if (!userId) return;

    const [cachedMessages, cachedOutbox] = await Promise.all([
        loadCachedMessages(userId),
        loadOutbox(userId),
    ]);

    setMessages(sortMessages(cachedMessages));
    setOutbox(cachedOutbox);
}

export function startOutboxAutoSync(): () => void {
    if (typeof window === 'undefined') {
        return () => {};
    }

    if (stopAutoSyncHandler) {
        return stopAutoSyncHandler;
    }

    const replayIfPossible = () => {
        if (isOnline()) {
            void syncOutbox();
        }
    };

    const onOnline = () => replayIfPossible();
    const onVisibilityChange = () => {
        if (document.visibilityState === 'visible') {
            replayIfPossible();
        }
    };

    window.addEventListener('online', onOnline);
    document.addEventListener('visibilitychange', onVisibilityChange);

    stopAutoSyncHandler = () => {
        window.removeEventListener('online', onOnline);
        document.removeEventListener('visibilitychange', onVisibilityChange);
        stopAutoSyncHandler = null;
    };

    return stopAutoSyncHandler;
}

export function stopOutboxAutoSync(): void {
    if (stopAutoSyncHandler) {
        stopAutoSyncHandler();
    }
}

// Actions
export async function fetchMessages(since?: string): Promise<void> {
    if (!isOnline()) {
        return;
    }

    setIsSyncing(true);
    try {
        const response = await api.getMessages(since);
        if (since) {
            mergeSyncedIntoMessages(response.messages);
        } else {
            setMessages(sortMessages(response.messages.map((msg) => ({
                ...msg,
                syncState: 'synced',
                localUpdatedAt: msg.updated_at,
            }))));
        }
        setLastSync(getNowIso());
        await persistMessages();
    } finally {
        setIsSyncing(false);
    }
}

export async function addMessage(content: string, clientId?: string): Promise<Message> {
    const id = clientId ?? createMessageId();
    const now = getNowIso();
    const userId = getCurrentUserId() ?? 'local-user';

    const optimisticMessage: Message = {
        id,
        user_id: userId,
        content,
        created_at: now,
        updated_at: now,
        syncState: 'pending',
        localUpdatedAt: now,
    };

    setMessages((prev) => sortMessages([optimisticMessage, ...prev.filter((m) => m.id !== id)]));

    await enqueueOperation({
        opId: createOperationId(),
        messageId: id,
        type: 'create',
        payload: { content, id },
        createdAt: now,
        updatedAt: now,
        status: 'pending',
        attempts: 0,
    });

    await persistMessages();

    if (isOnline()) {
        await syncOutbox();
    }

    return messages().find((m) => m.id === id) ?? optimisticMessage;
}

export async function updateMessage(id: string, content: string): Promise<Message> {
    const now = getNowIso();

    setMessages((prev) => prev.map((m) => (
        m.id === id
            ? { ...m, content, updated_at: now, syncState: 'pending', localUpdatedAt: now }
            : m
    )));

    await enqueueOperation({
        opId: createOperationId(),
        messageId: id,
        type: 'update',
        payload: { content },
        createdAt: now,
        updatedAt: now,
        status: 'pending',
        attempts: 0,
    });

    await persistMessages();

    if (isOnline()) {
        await syncOutbox();
    }

    const updated = messages().find((m) => m.id === id);
    if (!updated) {
        throw new Error('Message not found');
    }
    return updated;
}

export async function deleteMessage(id: string): Promise<void> {
    const now = getNowIso();

    setMessages((prev) => prev.filter((m) => m.id !== id));

    await enqueueOperation({
        opId: createOperationId(),
        messageId: id,
        type: 'delete',
        payload: { content: '' },
        createdAt: now,
        updatedAt: now,
        status: 'pending',
        attempts: 0,
    });

    await persistMessages();

    if (isOnline()) {
        await syncOutbox();
    }
}

export async function syncOutbox(): Promise<void> {
    if (isReplaying() || !isOnline()) {
        return;
    }

    const userId = getCurrentUserId();
    if (!userId) {
        return;
    }

    setIsReplaying(true);
    setAuthRequiredForReplay(false);
    setLastReplayError(null);

    try {
        while (true) {
            const nextOp = outbox()
                .filter((op) => op.status === 'pending')
                .sort((a, b) => new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime())[0];

            if (!nextOp) {
                break;
            }

            markOperationStatus(nextOp.opId, {
                status: 'replaying',
                attempts: nextOp.attempts + 1,
                updatedAt: getNowIso(),
            });
            await persistOutbox();

            try {
                if (nextOp.type === 'create') {
                    const payload = nextOp.payload as { content: string; id?: string };
                    const created = await api.createMessage(payload.content, nextOp.messageId);
                    setMessages((prev) => sortMessages(prev.map((m) => (
                        m.id === nextOp.messageId
                            ? {
                                ...created,
                                syncState: 'synced',
                                localUpdatedAt: created.updated_at,
                            }
                            : m
                    ))));
                } else if (nextOp.type === 'update') {
                    const payload = nextOp.payload as { content: string };
                    const updated = await api.updateMessage(nextOp.messageId, payload.content);
                    setMessages((prev) => sortMessages(prev.map((m) => (
                        m.id === nextOp.messageId
                            ? {
                                ...updated,
                                syncState: 'synced',
                                localUpdatedAt: updated.updated_at,
                            }
                            : m
                    ))));
                } else {
                    await api.deleteMessage(nextOp.messageId);
                    setMessages((prev) => prev.filter((m) => m.id !== nextOp.messageId));
                }

                removeOperation(nextOp.opId);
                await Promise.all([persistMessages(), persistOutbox()]);
                setLastReplayAt(getNowIso());
            } catch (error) {
                if (isNetworkError(error)) {
                    markOperationStatus(nextOp.opId, {
                        status: 'pending',
                        lastError: 'Network unavailable',
                        updatedAt: getNowIso(),
                    });
                    await persistOutbox();
                    setLastReplayError('Network unavailable');
                    break;
                }

                if (error instanceof ApiError && (error.status === 401 || error.status === 403)) {
                    markOperationStatus(nextOp.opId, {
                        status: 'pending',
                        lastError: 'Authentication required',
                        lastHttpStatus: error.status,
                        updatedAt: getNowIso(),
                    });
                    await persistOutbox();
                    setLastReplayError('Authentication required');
                    setAuthRequiredForReplay(true);
                    break;
                }

                markOperationStatus(nextOp.opId, {
                    status: 'failed',
                    lastError: error instanceof Error ? error.message : 'Replay failed',
                    lastHttpStatus: error instanceof ApiError ? error.status : undefined,
                    updatedAt: getNowIso(),
                });
                setMessageSyncState(nextOp.messageId, 'failed');
                await Promise.all([persistMessages(), persistOutbox()]);
            }
        }
    } finally {
        setIsReplaying(false);
    }
}

export async function retryFailedOperation(opId: string): Promise<void> {
    markOperationStatus(opId, {
        status: 'pending',
        lastError: undefined,
        lastHttpStatus: undefined,
        updatedAt: getNowIso(),
    });
    await persistOutbox();

    const op = outbox().find((candidate) => candidate.opId === opId);
    if (op) {
        setMessageSyncState(op.messageId, 'pending');
        await persistMessages();
    }

    if (isOnline()) {
        await syncOutbox();
    }
}

export async function discardFailedOperation(opId: string): Promise<void> {
    const op = outbox().find((candidate) => candidate.opId === opId);
    removeOperation(opId);
    await persistOutbox();

    if (op && op.type !== 'delete') {
        setMessageSyncState(op.messageId, 'synced');
        await persistMessages();
    }
}

// Optimistic update helpers (for compatibility with existing tests/flows)
export function optimisticAdd(message: Message): void {
    setMessages((prev) => sortMessages([message, ...prev]));
}

export function optimisticUpdate(id: string, content: string): void {
    setMessages((prev) =>
        prev.map((m) => m.id === id ? { ...m, content, updated_at: getNowIso() } : m)
    );
}

export function optimisticDelete(id: string): void {
    setMessages((prev) => prev.filter((m) => m.id !== id));
}

export async function clearOfflineForCurrentUser(): Promise<void> {
    const userId = getCurrentUserId();
    if (userId) {
        await purgeOfflineDataForUser(userId);
    }
}

export function clearMessages(): void {
    setMessages([]);
    setOutbox([]);
    setLastSync(null);
    setLastReplayAt(null);
    setLastReplayError(null);
    setAuthRequiredForReplay(false);
}
