import { createSignal } from 'solid-js';
import { api } from '../services/api';
import type { User } from '../types';
import { clearOfflineForCurrentUser, clearMessages, stopOutboxAutoSync } from './messagesStore';

const TOKEN_KEY = 'token';
const USER_KEY = 'user';
const CURRENT_USER_ID_KEY = 'current_user_id';

// Auth state
const [token, setTokenSignal] = createSignal<string | null>(
    typeof window !== 'undefined' ? localStorage.getItem(TOKEN_KEY) : null
);
const [user, setUserSignal] = createSignal<User | null>(
    typeof window !== 'undefined'
        ? (() => {
            const serialized = localStorage.getItem(USER_KEY);
            if (!serialized) return null;
            try {
                return JSON.parse(serialized) as User;
            } catch {
                return null;
            }
        })()
        : null
);

// Derived state
export function isAuthenticated(): boolean {
    return token() !== null;
}

// Store object for reactive access
export const authStore = {
    get token() { return token(); },
    get user() { return user(); },
};

// Actions
export function setToken(newToken: string | null): void {
    setTokenSignal(newToken);
    if (newToken) {
        localStorage.setItem(TOKEN_KEY, newToken);
    } else {
        localStorage.removeItem(TOKEN_KEY);
    }
}

export function setUser(newUser: User | null): void {
    setUserSignal(newUser);
    if (newUser) {
        localStorage.setItem(USER_KEY, JSON.stringify(newUser));
        localStorage.setItem(CURRENT_USER_ID_KEY, newUser.id);
    } else {
        localStorage.removeItem(USER_KEY);
        localStorage.removeItem(CURRENT_USER_ID_KEY);
    }
}

export async function login(email: string, password: string): Promise<void> {
    const response = await api.login(email, password);
    setToken(response.token);
    setUser(response.user);
}

export async function logout(): Promise<void> {
    stopOutboxAutoSync();
    const purgePromise = clearOfflineForCurrentUser();
    clearMessages();
    setToken(null);
    setUser(null);
    await purgePromise;
}
