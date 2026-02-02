import { Component, createSignal, createEffect, onMount } from 'solid-js';
import './MessageInput.css';

interface MessageInputProps {
    onSubmit: (content: string) => void;
    disabled?: boolean;
    placeholder?: string;
    initialValue?: string;
}

export const MessageInput: Component<MessageInputProps> = (props) => {
    let textareaRef: HTMLTextAreaElement | undefined;

    const [content, setContent] = createSignal(props.initialValue || '');
    const [isFocused, setIsFocused] = createSignal(false);

    // Auto-resize textarea
    const adjustHeight = () => {
        if (textareaRef) {
            textareaRef.style.height = 'auto';
            textareaRef.style.height = `${Math.min(textareaRef.scrollHeight, 200)}px`;
        }
    };

    createEffect(() => {
        content(); // Track content changes
        adjustHeight();
    });

    const handleSubmit = () => {
        const trimmed = content().trim();
        if (trimmed && !props.disabled) {
            props.onSubmit(trimmed);
            setContent('');
        }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
        // Submit on Enter, new line on Shift+Enter
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            handleSubmit();
        }
    };

    return (
        <div class={`message-input-container ${isFocused() ? 'focused' : ''}`}>
            <textarea
                ref={textareaRef}
                class="message-input"
                value={content()}
                onInput={(e) => setContent(e.currentTarget.value)}
                onKeyDown={handleKeyDown}
                onFocus={() => setIsFocused(true)}
                onBlur={() => setIsFocused(false)}
                placeholder={props.placeholder || "What's on your mind?"}
                disabled={props.disabled}
                rows={1}
                aria-label="Message content"
            />

            <button
                class="submit-button"
                onClick={handleSubmit}
                disabled={!content().trim() || props.disabled}
                aria-label="Send message"
            >
                <svg
                    viewBox="0 0 24 24"
                    class="submit-icon"
                    fill="currentColor"
                >
                    <path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" />
                </svg>
            </button>
        </div>
    );
};
