import type { SearchQuery } from '../types';

/**
 * Parses search query with special syntax:
 * - tag:name or tag:name1,name2 for hashtags
 * - date:MM/DD/YY-MM/DD/YY for date ranges
 * - Everything else is text search
 */
export function parseSearchQuery(input: string): SearchQuery {
    const query: SearchQuery = {};
    let remaining = input;

    // Extract tag:value
    const tagMatch = remaining.match(/tag:([^\s]+)/i);
    if (tagMatch) {
        query.tags = tagMatch[1].replace(/,/g, ',');
        remaining = remaining.replace(tagMatch[0], '').trim();
    }

    // Extract date:MM/DD/YY-MM/DD/YY
    const dateMatch = remaining.match(/date:(\d{1,2}\/\d{1,2}\/\d{2,4})-(\d{1,2}\/\d{1,2}\/\d{2,4})/i);
    if (dateMatch) {
        const fromDate = parseDate(dateMatch[1]);
        const toDate = parseDate(dateMatch[2]);
        if (fromDate) query.from = fromDate.toISOString();
        if (toDate) {
            toDate.setHours(23, 59, 59, 999);
            query.to = toDate.toISOString();
        }
        remaining = remaining.replace(dateMatch[0], '').trim();
    }

    // Remaining text is the search query
    if (remaining.trim()) {
        query.q = remaining.trim();
    }

    return query;
}

function parseDate(dateStr: string): Date | null {
    const parts = dateStr.split('/');
    if (parts.length !== 3) return null;

    const month = parseInt(parts[0], 10) - 1; // 0-indexed
    const day = parseInt(parts[1], 10);
    let year = parseInt(parts[2], 10);

    // Handle 2-digit years
    if (year < 100) {
        year += year < 50 ? 2000 : 1900;
    }

    const date = new Date(year, month, day);
    if (isNaN(date.getTime())) return null;
    return date;
}
