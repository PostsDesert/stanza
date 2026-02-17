import { Component, createSignal, Show } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { authStore } from '../stores/authStore';
import { showToast } from '../stores/uiStore';
import { api } from '../services/api';
import { discardFailedOperation, messagesStore, retryFailedOperation, syncOutbox } from '../stores/messagesStore';
import { ThemeToggle } from '../components/ThemeToggle';
import { LoadingSpinner } from '../components/LoadingSpinner';
import { SyncStatus } from '../components/SyncStatus';
import './Settings.css';

export const Settings: Component = () => {
    const navigate = useNavigate();

    // Email form
    const [email, setEmail] = createSignal(authStore.user?.email || '');
    const [isUpdatingEmail, setIsUpdatingEmail] = createSignal(false);

    // Username form
    const [username, setUsername] = createSignal(authStore.user?.username || '');
    const [isUpdatingUsername, setIsUpdatingUsername] = createSignal(false);

    // Password form
    const [currentPassword, setCurrentPassword] = createSignal('');
    const [newPassword, setNewPassword] = createSignal('');
    const [confirmPassword, setConfirmPassword] = createSignal('');
    const [isUpdatingPassword, setIsUpdatingPassword] = createSignal(false);

    const handleUpdateEmail = async (e: Event) => {
        e.preventDefault();
        if (!email().trim()) return;

        setIsUpdatingEmail(true);
        try {
            await api.updateEmail(email());
            showToast('Email updated', 'success');
        } catch (err) {
            showToast('Failed to update email', 'error');
        } finally {
            setIsUpdatingEmail(false);
        }
    };

    const handleUpdateUsername = async (e: Event) => {
        e.preventDefault();
        if (!username().trim()) return;

        setIsUpdatingUsername(true);
        try {
            await api.updateUsername(username());
            showToast('Username updated', 'success');
        } catch (err) {
            showToast('Failed to update username', 'error');
        } finally {
            setIsUpdatingUsername(false);
        }
    };

    const handleUpdatePassword = async (e: Event) => {
        e.preventDefault();
        if (newPassword() !== confirmPassword()) {
            showToast('Passwords do not match', 'error');
            return;
        }
        if (newPassword().length < 8) {
            showToast('Password must be at least 8 characters', 'error');
            return;
        }

        setIsUpdatingPassword(true);
        try {
            await api.updatePassword(currentPassword(), newPassword());
            showToast('Password updated', 'success');
            setCurrentPassword('');
            setNewPassword('');
            setConfirmPassword('');
        } catch (err) {
            showToast('Failed to update password', 'error');
        } finally {
            setIsUpdatingPassword(false);
        }
    };

    const handleExportJson = async () => {
        try {
            const messages = await api.exportJson();
            const blob = new Blob([JSON.stringify(messages, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'dissipate-export.json';
            a.click();
            URL.revokeObjectURL(url);
            showToast('Export downloaded', 'success');
        } catch (err) {
            showToast('Failed to export', 'error');
        }
    };

    const handleExportMarkdown = async () => {
        try {
            const markdown = await api.exportMarkdown();
            const blob = new Blob([markdown], { type: 'text/markdown' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = 'dissipate-export.md';
            a.click();
            URL.revokeObjectURL(url);
            showToast('Export downloaded', 'success');
        } catch (err) {
            showToast('Failed to export', 'error');
        }
    };

    const handleRetryFailed = async (opId: string) => {
        try {
            await retryFailedOperation(opId);
            showToast('Retry scheduled', 'info');
            await syncOutbox();
        } catch (err) {
            showToast('Failed to retry operation', 'error');
        }
    };

    const handleDiscardFailed = async (opId: string) => {
        try {
            await discardFailedOperation(opId);
            showToast('Failed operation discarded', 'info');
        } catch (err) {
            showToast('Failed to discard operation', 'error');
        }
    };

    return (
        <div class="settings-page">
            <header class="settings-header">
                <button
                    class="back-button"
                    onClick={() => navigate('/')}
                    aria-label="Back to feed"
                >
                    ← Back
                </button>
                <h1 class="settings-title">Settings</h1>
                <SyncStatus />
                <ThemeToggle />
            </header>

            <main class="settings-main">
                <section class="settings-section">
                    <h2>Email</h2>
                    <form onSubmit={handleUpdateEmail}>
                        <input
                            type="email"
                            class="form-input"
                            value={email()}
                            onInput={(e) => setEmail(e.currentTarget.value)}
                            placeholder="Email address"
                            disabled={isUpdatingEmail()}
                        />
                        <button type="submit" class="form-button" disabled={isUpdatingEmail()}>
                            <Show when={isUpdatingEmail()} fallback="Update Email">
                                <LoadingSpinner size="sm" /> Updating...
                            </Show>
                        </button>
                    </form>
                </section>

                <section class="settings-section">
                    <h2>Username</h2>
                    <form onSubmit={handleUpdateUsername}>
                        <input
                            type="text"
                            class="form-input"
                            value={username()}
                            onInput={(e) => setUsername(e.currentTarget.value)}
                            placeholder="Username"
                            disabled={isUpdatingUsername()}
                        />
                        <button type="submit" class="form-button" disabled={isUpdatingUsername()}>
                            <Show when={isUpdatingUsername()} fallback="Update Username">
                                <LoadingSpinner size="sm" /> Updating...
                            </Show>
                        </button>
                    </form>
                </section>

                <section class="settings-section">
                    <h2>Change Password</h2>
                    <form onSubmit={handleUpdatePassword}>
                        <input
                            type="password"
                            class="form-input"
                            value={currentPassword()}
                            onInput={(e) => setCurrentPassword(e.currentTarget.value)}
                            placeholder="Current password"
                            disabled={isUpdatingPassword()}
                        />
                        <input
                            type="password"
                            class="form-input"
                            value={newPassword()}
                            onInput={(e) => setNewPassword(e.currentTarget.value)}
                            placeholder="New password"
                            disabled={isUpdatingPassword()}
                        />
                        <input
                            type="password"
                            class="form-input"
                            value={confirmPassword()}
                            onInput={(e) => setConfirmPassword(e.currentTarget.value)}
                            placeholder="Confirm new password"
                            disabled={isUpdatingPassword()}
                        />
                        <button type="submit" class="form-button" disabled={isUpdatingPassword()}>
                            <Show when={isUpdatingPassword()} fallback="Update Password">
                                <LoadingSpinner size="sm" /> Updating...
                            </Show>
                        </button>
                    </form>
                </section>

                <section class="settings-section">
                    <h2>Export Data</h2>
                    <div class="export-buttons">
                        <button class="form-button" onClick={handleExportJson}>
                            Export as JSON
                        </button>
                        <button class="form-button" onClick={handleExportMarkdown}>
                            Export as Markdown
                        </button>
                    </div>
                </section>

                <section class="settings-section">
                    <h2>Offline Sync Issues</h2>
                    <Show
                        when={messagesStore.failedOperations.length > 0}
                        fallback={<p>No failed sync operations.</p>}
                    >
                        <div class="failed-ops-list">
                            {messagesStore.failedOperations.map((op) => (
                                <div class="failed-op-item">
                                    <div class="failed-op-text">
                                        <strong>{op.type.toUpperCase()}</strong> for message {op.messageId}
                                        <div>{op.lastError || 'Unknown error'}</div>
                                    </div>
                                    <div class="failed-op-actions">
                                        <button
                                            class="form-button"
                                            type="button"
                                            onClick={() => handleRetryFailed(op.opId)}
                                        >
                                            Retry
                                        </button>
                                        <button
                                            class="form-button"
                                            type="button"
                                            onClick={() => handleDiscardFailed(op.opId)}
                                        >
                                            Discard
                                        </button>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </Show>
                </section>
            </main>
        </div>
    );
};

export default Settings;
