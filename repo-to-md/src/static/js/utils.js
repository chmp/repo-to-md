/**
 * Shared utility functions
 * Pure functions extracted from components for testability
 */

/**
 * Escape HTML special characters to prevent XSS
 * @param {string} text - Text to escape
 * @returns {string} Escaped text
 */
export function escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * Escape text for use in HTML attributes
 * @param {string} text - Text to escape
 * @returns {string} Escaped text
 */
export function escapeAttr(text) {
    if (!text) return '';
    return text.replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

/**
 * Extract filename from a path
 * @param {string} path - File path
 * @returns {string} Filename
 */
export function getFileName(path) {
    return path.split('/').pop() || path;
}

/**
 * Format a Unix timestamp to a locale string
 * @param {number|string} timestamp - Unix timestamp in seconds
 * @returns {string} Formatted date string
 */
export function formatDate(timestamp) {
    // Handle Unix timestamp (seconds)
    const date = new Date(parseInt(timestamp) * 1000);
    if (isNaN(date.getTime())) {
        return timestamp;
    }
    return date.toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
    });
}

/**
 * Classify a diff row type based on its line data
 * @param {Object} row - Diff row with old_line and new_line properties
 * @returns {string} Row type: 'modified', 'context', 'deletion', or 'addition'
 */
export function getRowType(row) {
    if (row.old_line && row.new_line) {
        if (row.old_line.line_type === 'deletion' && row.new_line.line_type === 'addition') {
            return 'modified';
        }
        return 'context';
    }
    if (row.old_line && row.old_line.line_type === 'deletion') {
        return 'deletion';
    }
    if (row.new_line && row.new_line.line_type === 'addition') {
        return 'addition';
    }
    return 'context';
}

/**
 * Group comments by their line number
 * @param {Array} comments - Array of comment objects with line property
 * @returns {Object} Object mapping line numbers to arrays of comments
 */
export function groupCommentsByLine(comments) {
    const result = {};
    for (const comment of comments) {
        if (!result[comment.line]) {
            result[comment.line] = [];
        }
        result[comment.line].push(comment);
    }
    return result;
}

/**
 * Group comments by file path, with global comments under '__general__'
 * @param {Array} comments - Array of comment objects with path and line properties
 * @returns {Object} Object mapping file paths to arrays of comments
 */
export function getCommentsByFile(comments) {
    const result = { '__general__': [] };
    for (const comment of comments) {
        const key = comment.line === null ? '__general__' : comment.path;
        if (!result[key]) {
            result[key] = [];
        }
        result[key].push(comment);
    }
    return result;
}

/**
 * Compute minimal file tree items from full file objects
 * @param {Array} files - Array of file objects with path and status
 * @param {Object} commentsByFile - Object mapping file paths to comment arrays
 * @param {Set} viewedFiles - Set of viewed file paths
 * @returns {Array} Array of { path, status, commentCount, isViewed }
 */
export function computeFileTreeItems(files, commentsByFile, viewedFiles) {
    return files.map(file => ({
        path: file.path,
        status: file.status,
        commentCount: (commentsByFile[file.path] || []).filter(c => !c.is_minimized).length,
        isViewed: viewedFiles.has(file.path),
    }));
}

/**
 * Get an ordered list of comment positions for navigation
 * @param {Array} files - Array of file objects with path property
 * @param {Object} commentsByFile - Object mapping file paths to comment arrays
 * @param {boolean} skipMinimized - Whether to skip minimized comments
 * @returns {Array} Array of { path, line, id } in navigation order
 */
export function getCommentPositions(files, commentsByFile, skipMinimized = false) {
    const positions = [];

    // Global comments first
    for (const c of (commentsByFile['__general__'] || [])) {
        if (skipMinimized && c.is_minimized) continue;
        positions.push({ path: '__general__', line: null, id: c.id });
    }

    // File comments in file order, sorted by line within each file
    for (const file of files) {
        const fileComments = (commentsByFile[file.path] || [])
            .filter(c => !skipMinimized || !c.is_minimized)
            .sort((a, b) => (a.line || 0) - (b.line || 0));
        for (const c of fileComments) {
            positions.push({ path: file.path, line: c.line, id: c.id });
        }
    }

    return positions;
}

/**
 * Setup auto-resize behavior for a textarea
 * @param {HTMLTextAreaElement} textarea - The textarea element
 */
export function setupAutoResizeTextarea(textarea) {
    textarea.style.height = `${textarea.scrollHeight + 2}px`;
    textarea.addEventListener('input', () => {
        textarea.style.height = 'auto';
        textarea.style.height = `${textarea.scrollHeight + 2}px`;
    });
}

/**
 * Setup keyboard shortcuts for a textarea (Ctrl/Cmd+Enter to submit, Escape to cancel)
 * @param {HTMLTextAreaElement} textarea - The textarea element
 * @param {Function} onSubmit - Callback for submit action
 * @param {Function} onCancel - Callback for cancel action
 */
export function setupTextareaKeyboardShortcuts(textarea, onSubmit, onCancel) {
    textarea.addEventListener('keydown', (e) => {
        if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
            e.preventDefault();
            onSubmit();
        }
        if (e.key === 'Escape') {
            e.preventDefault();
            onCancel();
        }
    });
}
