import { createEffect, onCleanup, type Component } from 'solid-js';
import { Router, Route, Navigate } from '@solidjs/router';
import { isAuthenticated } from './stores/authStore';
import { ToastContainer } from './components/Toast';
import { initOfflineMessages, startOutboxAutoSync, stopOutboxAutoSync, syncOutbox } from './stores/messagesStore';
import Login from './pages/Login';
import Feed from './pages/Feed';
import Settings from './pages/Settings';
import PostDetail from './pages/PostDetail';
import './index.css';

// Protected route component
const ProtectedRoute: Component<{ component: Component }> = (props) => {
    if (!isAuthenticated()) {
        return <Navigate href="/login" />;
    }
    return <props.component />;
};

// Public route (redirects if already authenticated)
const PublicRoute: Component<{ component: Component }> = (props) => {
    if (isAuthenticated()) {
        return <Navigate href="/" />;
    }
    return <props.component />;
};

const App: Component = () => {
    let autoSyncCleanup: (() => void) | null = null;

    createEffect(() => {
        if (isAuthenticated()) {
            void initOfflineMessages();
            void syncOutbox();
            if (!autoSyncCleanup) {
                autoSyncCleanup = startOutboxAutoSync();
            }
        } else if (autoSyncCleanup) {
            autoSyncCleanup();
            autoSyncCleanup = null;
        }
    });

    onCleanup(() => {
        if (autoSyncCleanup) {
            autoSyncCleanup();
            autoSyncCleanup = null;
        }
        stopOutboxAutoSync();
    });

    return (
        <>
            <ToastContainer />
            <Router>
                <Route path="/login" component={() => <PublicRoute component={Login} />} />
                <Route path="/settings" component={() => <ProtectedRoute component={Settings} />} />
                <Route path="/post/:id" component={() => <ProtectedRoute component={PostDetail} />} />
                <Route path="/" component={() => <ProtectedRoute component={Feed} />} />
            </Router>
        </>
    );
};

export default App;
