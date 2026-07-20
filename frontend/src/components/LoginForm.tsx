import { Component, createSignal, Show } from 'solid-js';
import { login } from '../stores/authStore';
import { showToast } from '../stores/uiStore';
import { LoadingSpinner } from './LoadingSpinner';
import './LoginForm.css';

interface LoginFormProps {
    onSuccess?: () => void;
}

export const LoginForm: Component<LoginFormProps> = (props) => {
    const [email, setEmail] = createSignal('');
    const [password, setPassword] = createSignal('');
    const [isLoading, setIsLoading] = createSignal(false);
    const [error, setError] = createSignal<string | null>(null);

    const handleSubmit = async (e: Event) => {
        e.preventDefault();
        setError(null);

        if (!email().trim() || !password().trim()) {
            setError('Please enter both email and password');
            return;
        }

        setIsLoading(true);
        try {
            await login(email(), password());
            showToast('Login successful!', 'success');
            props.onSuccess?.();
        } catch (err) {
            const message = err instanceof Error ? err.message : 'Login failed';
            setError('Invalid email or password');
            showToast(message, 'error');
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <form class="login-form" onSubmit={handleSubmit}>
            <h1 class="login-title">Stanza</h1>
            <p class="login-subtitle">Your personal microblog</p>

            <Show when={error()}>
                <div class="login-error" role="alert">
                    {error()}
                </div>
            </Show>

            <div class="form-group">
                <label for="email" class="form-label">Email</label>
                <input
                    id="email"
                    type="email"
                    class="form-input"
                    value={email()}
                    onInput={(e) => setEmail(e.currentTarget.value)}
                    placeholder="you@example.com"
                    required
                    disabled={isLoading()}
                    autocomplete="email"
                />
            </div>

            <div class="form-group">
                <label for="password" class="form-label">Password</label>
                <input
                    id="password"
                    type="password"
                    class="form-input"
                    value={password()}
                    onInput={(e) => setPassword(e.currentTarget.value)}
                    placeholder="••••••••"
                    required
                    disabled={isLoading()}
                    autocomplete="current-password"
                />
            </div>

            <button
                type="submit"
                class="login-button"
                disabled={isLoading()}
            >
                <Show when={isLoading()} fallback="Sign In">
                    <LoadingSpinner size="sm" />
                    <span>Signing in...</span>
                </Show>
            </button>

            <div class="login-footer">
                <button
                    type="button"
                    class="create-account-button"
                    disabled
                    title="Account creation coming soon"
                >
                    Create Account
                </button>
            </div>
        </form>
    );
};
