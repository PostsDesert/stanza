import { clear, createStore, del, get, set } from 'idb-keyval';
import type { Message, PendingOperation } from '../types';

const offlineDb = createStore('stanza-offline-db', 'offline-kv');
const hasIndexedDb = typeof indexedDB !== 'undefined';

function messagesKey(userId: string): string {
    return `messages:${userId}`;
}

function outboxKey(userId: string): string {
    return `outbox:${userId}`;
}

export async function loadCachedMessages(userId: string): Promise<Message[]> {
    if (!hasIndexedDb) return [];
    const cached = await get<Message[]>(messagesKey(userId), offlineDb);
    return Array.isArray(cached) ? cached : [];
}

export async function saveCachedMessages(userId: string, messages: Message[]): Promise<void> {
    if (!hasIndexedDb) return;
    await set(messagesKey(userId), messages, offlineDb);
}

export async function loadOutbox(userId: string): Promise<PendingOperation[]> {
    if (!hasIndexedDb) return [];
    const outbox = await get<PendingOperation[]>(outboxKey(userId), offlineDb);
    return Array.isArray(outbox) ? outbox : [];
}

export async function saveOutbox(userId: string, outbox: PendingOperation[]): Promise<void> {
    if (!hasIndexedDb) return;
    await set(outboxKey(userId), outbox, offlineDb);
}

export async function purgeOfflineDataForUser(userId: string): Promise<void> {
    if (!hasIndexedDb) return;
    await Promise.all([
        del(messagesKey(userId), offlineDb),
        del(outboxKey(userId), offlineDb),
    ]);
}

export async function purgeAllOfflineData(): Promise<void> {
    if (!hasIndexedDb) return;
    await clear(offlineDb);
}
