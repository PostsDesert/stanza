import { Component } from 'solid-js';
import { uiStore, cycleTheme } from '../stores/uiStore';
import { SunIcon } from './icons/SunIcon';
import { MoonIcon } from './icons/MoonIcon';
import { MonitorIcon } from './icons/MonitorIcon';
import './ThemeToggle.css';

export const ThemeToggle: Component = () => {
    const getIcon = () => {
        switch (uiStore.theme) {
            case 'light': return <SunIcon width="20" height="20" />;
            case 'dark': return <MoonIcon width="20" height="20" />;
            default: return <MonitorIcon width="20" height="20" />;
        }
    };

    const getLabel = () => {
        switch (uiStore.theme) {
            case 'light': return 'Light mode';
            case 'dark': return 'Dark mode';
            default: return 'Auto mode';
        }
    };

    return (
        <button
            class="theme-toggle"
            onClick={cycleTheme}
            aria-label={`Current theme: ${getLabel()}. Click to change.`}
            title={getLabel()}
        >
            <span class="theme-icon">{getIcon()}</span>
        </button>
    );
};
