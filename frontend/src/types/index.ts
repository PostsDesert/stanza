// User type (without password_hash and salt for frontend)
export interface User {
    id: string;
    email: string;
    username: string;
    created_at: string;
    updated_at: string;
}

// Message type
export interface Message {
    id: string;
    user_id: string;
    content: string;
    created_at: string;
    updated_at: string;
}

// Auth types
export interface LoginRequest {
    email: string;
    password: string;
}

export interface LoginResponse {
    token: string;
    user: User;
}

// Message types
export interface CreateMessageRequest {
    id?: string;
    content: string;
}

export interface UpdateMessageRequest {
    content: string;
}

export interface MessagesQuery {
    since?: string;
}

export interface MessagesResponse {
    messages: Message[];
}

// Search types
export interface SearchQuery {
    q?: string;
    from?: string;
    to?: string;
    tags?: string;
}

export interface SearchResponse {
    messages: Message[];
    total: number;
}

// User update types
export interface UpdateEmailRequest {
    email: string;
}

export interface UpdateUsernameRequest {
    username: string;
}

export interface UpdatePasswordRequest {
    current_password: string;
    new_password: string;
}

// Common response types
export interface SuccessResponse {
    success: boolean;
}

export interface ErrorResponse {
    error: string;
}

// Theme type
export type Theme = 'auto' | 'light' | 'dark';

// Pending operation for offline queue
export interface PendingOperation {
    id: string;
    type: 'create' | 'update' | 'delete';
    data: CreateMessageRequest | UpdateMessageRequest | string;
    timestamp: string;
    retries: number;
}

// Toast notification type
export interface Toast {
    id: string;
    message: string;
    type: 'success' | 'error' | 'info';
    duration?: number;
}
