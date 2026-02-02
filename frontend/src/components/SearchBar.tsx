import { Component, Show } from 'solid-js';
import type { SearchQuery } from '../types';
import { parseSearchQuery } from '../utils/search';
import './SearchBar.css';

interface SearchBarProps {
    query: string;
    onQueryChange: (query: string) => void;
    onSearch: (query: SearchQuery) => void;
    onClear: () => void;
    isSearching: boolean;
    isSearchActive: boolean;
}

export const SearchBar: Component<SearchBarProps> = (props) => {
    const handleSearch = () => {
        const trimmed = props.query.trim();
        if (!trimmed) return;

        const query = parseSearchQuery(trimmed);
        if (query.q || query.tags || query.from || query.to) {
            props.onSearch(query);
        }
    };

    const handleClear = () => {
        props.onQueryChange('');
        props.onClear();
    };

    const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            handleSearch();
        }
        if (e.key === 'Escape') {
            handleClear();
        }
    };

    return (
        <div class="search-bar">
            <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="11" cy="11" r="8" />
                <path d="M21 21l-4.35-4.35" />
            </svg>
            <input
                type="text"
                class="search-input"
                placeholder="Search... (tag:name date:MM/DD/YY-MM/DD/YY)"
                value={props.query}
                onInput={(e) => props.onQueryChange(e.currentTarget.value)}
                onKeyDown={handleKeyDown}
            />
            <Show when={props.query || props.isSearchActive}>
                <button
                    class="search-clear"
                    onClick={handleClear}
                    aria-label="Clear search"
                >
                    <svg viewBox="0 0 24 24" fill="currentColor">
                        <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z" />
                    </svg>
                </button>
            </Show>
            <Show when={props.isSearching}>
                <div class="search-spinner" />
            </Show>
        </div>
    );
};
