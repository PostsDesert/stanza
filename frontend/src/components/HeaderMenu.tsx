import { Component, createSignal, Show, onCleanup } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { logout } from '../stores/authStore';
import { uiStore, cycleTheme } from '../stores/uiStore';
import { SettingsIcon } from './icons/SettingsIcon';
import { LogoutIcon } from './icons/LogoutIcon';
import { SunIcon } from './icons/SunIcon';
import { MoonIcon } from './icons/MoonIcon';
import { MonitorIcon } from './icons/MonitorIcon';
import { SyncStatus } from './SyncStatus';
import './HeaderMenu.css';

export const HeaderMenu: Component = () => {
    const navigate = useNavigate();
    const [isOpen, setIsOpen] = createSignal(false);

    const handleClickOutside = (e: MouseEvent) => {
        const target = e.target as HTMLElement;
        if (!target.closest('.header-menu')) {
            setIsOpen(false);
        }
    };

    const toggleMenu = () => {
        const newState = !isOpen();
        setIsOpen(newState);
        if (newState) {
            document.addEventListener('click', handleClickOutside);
        } else {
            document.removeEventListener('click', handleClickOutside);
        }
    };

    onCleanup(() => {
        document.removeEventListener('click', handleClickOutside);
    });

    const handleSettings = () => {
        setIsOpen(false);
        navigate('/settings');
    };

    const handleLogout = () => {
        setIsOpen(false);
        void logout();
        navigate('/login', { replace: true });
    };

    const handleThemeClick = () => {
        cycleTheme();
    };

    const getThemeIcon = () => {
        switch (uiStore.theme) {
            case 'light': return <SunIcon width="20" height="20" />;
            case 'dark': return <MoonIcon width="20" height="20" />;
            default: return <MonitorIcon width="20" height="20" />;
        }
    };

    const getThemeLabel = () => {
        switch (uiStore.theme) {
            case 'light': return 'Light';
            case 'dark': return 'Dark';
            default: return 'Auto';
        }
    };

    return (
        <div class="header-menu">
            <button
                class="menu-trigger"
                onClick={toggleMenu}
                aria-label="Menu"
                aria-expanded={isOpen()}
            >
                <svg viewBox="0 0 24 24" fill="currentColor">
                    <circle cx="12" cy="5" r="2" />
                    <circle cx="12" cy="12" r="2" />
                    <circle cx="12" cy="19" r="2" />
                </svg>
            </button>

            <Show when={isOpen()}>
                <div class="menu-dropdown">
                    <div class="menu-sync-status">
                        <SyncStatus variant="menu" />
                    </div>
                    <div class="menu-divider menu-sync-divider-mobile" />
                    <button class="menu-item menu-item-settings" onClick={handleSettings}>
                        <span class="menu-icon"><SettingsIcon width="20" height="20" /></span>
                        <span>Settings</span>
                    </button>
                    <button class="menu-item menu-item-theme" onClick={handleThemeClick}>
                        <span class="menu-icon">{getThemeIcon()}</span>
                        <span>Theme: {getThemeLabel()}</span>
                    </button>
                    <div class="menu-divider" />
                    <button class="menu-item menu-item-danger" onClick={handleLogout}>
                        <span class="menu-icon"><LogoutIcon width="20" height="20" /></span>
                        <span>Logout</span>
                    </button>
                </div>
            </Show>
        </div>
    );
};
