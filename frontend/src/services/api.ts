import type {
    LoginRequest,
    LoginResponse,
    Message,
    MessagesResponse,
    CreateMessageRequest,
    UpdateMessageRequest,
    UpdateEmailRequest,
    UpdateUsernameRequest,
    UpdatePasswordRequest,
    SuccessResponse,
    SearchQuery,
    SearchResponse,
} from '../types';

const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';

export class ApiError extends Error {
    status: number;

    constructor(message: string, status: number) {
        super(message);
        this.name = 'ApiError';
        this.status = status;
    }
}

function getToken(): string | null {
    return localStorage.getItem('token');
}

async function request<T>(
    endpoint: string,
    options: RequestInit = {}
): Promise<T> {
    const token = getToken();

    const headers: HeadersInit = {
        'Content-Type': 'application/json',
        ...options.headers,
    };

    if (token) {
        (headers as Record<string, string>)['Authorization'] = `Bearer ${token}`;
    }

    const response = await fetch(`${API_URL}${endpoint}`, {
        ...options,
        headers,
    });

    if (!response.ok) {
        throw new ApiError(response.statusText, response.status);
    }

    return response.json();
}

export const api = {
    // Auth
    async login(email: string, password: string): Promise<LoginResponse> {
        const body: LoginRequest = { email, password };
        return request<LoginResponse>('/login', {
            method: 'POST',
            body: JSON.stringify(body),
        });
    },

    // Messages
    async getMessages(since?: string): Promise<MessagesResponse> {
        const params = since ? `?since=${encodeURIComponent(since)}` : '';
        return request<MessagesResponse>(`/messages${params}`);
    },

    async searchMessages(query: SearchQuery): Promise<SearchResponse> {
        const params = new URLSearchParams();
        if (query.q) params.append('q', query.q);
        if (query.from) params.append('from', query.from);
        if (query.to) params.append('to', query.to);
        if (query.tags) params.append('tags', query.tags);
        const queryString = params.toString();
        return request<SearchResponse>(`/messages/search${queryString ? `?${queryString}` : ''}`);
    },

    async createMessage(content: string, id?: string): Promise<Message> {
        const body: CreateMessageRequest = { content };
        if (id) {
            body.id = id;
        }
        return request<Message>('/messages', {
            method: 'POST',
            body: JSON.stringify(body),
        });
    },

    async updateMessage(id: string, content: string): Promise<Message> {
        const body: UpdateMessageRequest = { content };
        return request<Message>(`/messages/${id}`, {
            method: 'PUT',
            body: JSON.stringify(body),
        });
    },

    async deleteMessage(id: string): Promise<SuccessResponse> {
        return request<SuccessResponse>(`/messages/${id}`, {
            method: 'DELETE',
        });
    },

    // User updates
    async updateEmail(email: string): Promise<SuccessResponse> {
        const body: UpdateEmailRequest = { email };
        return request<SuccessResponse>('/user/email', {
            method: 'PUT',
            body: JSON.stringify(body),
        });
    },

    async updateUsername(username: string): Promise<SuccessResponse> {
        const body: UpdateUsernameRequest = { username };
        return request<SuccessResponse>('/user/username', {
            method: 'PUT',
            body: JSON.stringify(body),
        });
    },

    async updatePassword(currentPassword: string, newPassword: string): Promise<SuccessResponse> {
        const body: UpdatePasswordRequest = {
            current_password: currentPassword,
            new_password: newPassword,
        };
        return request<SuccessResponse>('/user/password', {
            method: 'PUT',
            body: JSON.stringify(body),
        });
    },

    // Exports
    async exportJson(): Promise<Message[]> {
        return request<Message[]>('/export/json');
    },

    async exportMarkdown(): Promise<string> {
        const token = getToken();
        const headers: HeadersInit = {};
        if (token) {
            headers['Authorization'] = `Bearer ${token}`;
        }

        const response = await fetch(`${API_URL}/export/markdown`, { headers });
        if (!response.ok) {
            throw new ApiError(response.statusText, response.status);
        }
        return response.text();
    },
};
