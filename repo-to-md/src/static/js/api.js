/**
 * API client for the repo-to-md webdiff server
 */

const API_BASE = '/api/v1';
const NO_CONTENT = 204;

/**
 * Generic API request helper
 * @param {string} endpoint - API endpoint (without base)
 * @param {Object} options - Fetch options
 * @returns {Promise<Object|null>} Response JSON or null for DELETE
 */
async function apiRequest(endpoint, options = {}) {
    const response = await fetch(`${API_BASE}${endpoint}`, {
        headers: { 'Content-Type': 'application/json' },
        ...options,
    });
    if (!response.ok) {
        throw new Error(`API error: ${response.statusText}`);
    }
    if (response.status === NO_CONTENT) {
        return null;
    }
    return response.json();
}

/**
 * Fetch all session data (diff, comments, viewed files)
 * @returns {Promise<{start_ref: string, end_ref: string, files: Array, comments: Array, viewed_files: string[]}>}
 */
export async function fetchSession() {
    return apiRequest('/session');
}

/**
 * Create a new comment
 * @param {Object} comment - The comment data
 * @returns {Promise<{comment: Object}>}
 */
export async function createComment(comment) {
    return apiRequest('/comments', {
        method: 'POST',
        body: JSON.stringify(comment),
    });
}

/**
 * Update an existing comment
 * @param {string} id - Comment ID
 * @param {string} body - New comment text
 * @returns {Promise<{comment: Object}>}
 */
export async function updateComment(id, body) {
    return apiRequest(`/comments/${id}`, {
        method: 'PUT',
        body: JSON.stringify({ body }),
    });
}

/**
 * Delete a comment
 * @param {string} id - Comment ID
 * @returns {Promise<null>}
 */
export async function deleteComment(id) {
    return apiRequest(`/comments/${id}`, { method: 'DELETE' });
}

/**
 * Toggle the minimized state of a comment
 * @param {string} id - Comment ID
 * @returns {Promise<{comment: Object}>}
 */
export async function toggleMinimizeComment(id) {
    return apiRequest(`/comments/${id}/minimize`, { method: 'POST' });
}

/**
 * Set the viewed status of a file
 * @param {string} path - File path
 * @param {boolean} viewed - Whether the file is viewed
 * @returns {Promise<null>}
 */
export async function setFileViewed(path, viewed) {
    return apiRequest(`/paths/${encodeURIComponent(path)}`, {
        method: 'POST',
        body: JSON.stringify({ viewed }),
    });
}

/**
 * Request server shutdown
 * @returns {Promise<null>}
 */
export async function shutdown() {
    return apiRequest('/shutdown', { method: 'POST' });
}
