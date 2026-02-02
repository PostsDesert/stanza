import { Component, Show, createMemo } from 'solid-js';
import type { Message } from '../types';
import { formatDate, formatRelativeTime, isWithinMinutes } from '../utils/date';
import { EditIcon } from './icons/EditIcon';
import { DeleteIcon } from './icons/DeleteIcon';
import './MessageCard.css';

interface MessageCardProps {
    message: Message;
    onClick?: () => void;
    onEdit?: () => void;
    onDelete?: () => void;
    onTagClick?: (tag: string) => void;
}

const PREVIEW_LENGTH = 200;

export const MessageCard: Component<MessageCardProps> = (props) => {
    const preview = createMemo(() => {
        const content = props.message.content;
        if (content.length <= PREVIEW_LENGTH) {
            return { text: content, truncated: false, remaining: 0 };
        }
        return {
            text: content.slice(0, PREVIEW_LENGTH),
            truncated: true,
            remaining: content.length - PREVIEW_LENGTH,
        };
    });

    const formatContent = (text: string) => {
        const parts = text.split(/(#\w+)/g);
        return parts.map((part) => {
            if (part.startsWith('#') && part.length > 1) {
                const tag = part.substring(1);
                return (
                    <span
                        class="hashtag"
                        onClick={(e) => {
                            e.stopPropagation();
                            props.onTagClick?.(tag);
                        }}
                    >
                        {part}
                    </span>
                );
            }
            return part;
        });
    };

    const timestamp = createMemo(() => {
        // Show relative time if within last 24 hours, otherwise full date
        if (isWithinMinutes(props.message.created_at, 24 * 60)) {
            return formatRelativeTime(props.message.created_at);
        }
        return formatDate(props.message.created_at);
    });

    const handleEdit = (e: Event) => {
        e.stopPropagation();
        props.onEdit?.();
    };

    const handleDelete = (e: Event) => {
        e.stopPropagation();
        props.onDelete?.();
    };

    return (
        <article
            class="message-card"
            onClick={props.onClick}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => e.key === 'Enter' && props.onClick?.()}
        >
            <div class="message-content">
                <p class="message-text">
                    {formatContent(preview().text)}
                    <Show when={preview().truncated}>
                        <span class="message-truncated">
                            ... <span class="message-remaining">+{preview().remaining} chars</span>
                        </span>
                    </Show>
                </p>
            </div>

            <div class="message-footer">
                <time class="message-timestamp" datetime={props.message.created_at}>
                    {timestamp()}
                </time>

                <div class="message-actions">
                    <button
                        class="message-action"
                        onClick={handleEdit}
                        aria-label="Edit message"
                        title="Edit"
                    >
                        <EditIcon width="16" height="16" />
                    </button>
                    <button
                        class="message-action message-action-delete"
                        onClick={handleDelete}
                        aria-label="Delete message"
                        title="Delete"
                    >
                        <DeleteIcon width="16" height="16" />
                    </button>
                </div>
            </div>
        </article>
    );
};
