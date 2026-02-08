/**
 * <diff-view> custom element
 * Side-by-side diff view with comment support
 */

import './review-comment.js';
import './comment-form.js';
import { escapeHtml, escapeAttr, getRowType, groupCommentsByLine } from './utils.js';

class DiffView extends HTMLElement {
    constructor() {
        super();
        this.currentFile = null;      // Single file object
        this.currentComments = [];    // Comments for current file only
        this.globalComments = [];
        this.selectedFile = null;
        this.activeCommentForm = null; // { path, line, isGlobal }
        this.remainingUnviewed = 0;   // Count of unviewed files remaining
    }

    connectedCallback() {
        this.render();
    }

    /**
     * Set the current file to display with its comments
     * @param {Object|null} file - File diff object or null
     * @param {Array} comments - Comments for this file
     */
    setCurrentFile(file, comments) {
        this.currentFile = file;
        this.selectedFile = file?.path || null;
        this.currentComments = comments || [];
        this.activeCommentForm = null;
        this.render();
        this.scrollTop = 0;
    }

    /**
     * Set global comments (for __general__ section)
     * @param {Array} comments - Global comments array
     */
    setGlobalComments(comments) {
        this.globalComments = comments || [];
        if (this.selectedFile === '__general__') {
            this.render();
        }
    }

    /**
     * Update comments for the current file (without changing file)
     * @param {Array} comments - Updated comments array
     */
    updateCurrentComments(comments) {
        this.currentComments = comments || [];
        this.render();
    }

    /**
     * Show comment form at a specific line
     * @param {string} path - File path
     * @param {number} line - Line number
     * @param {string} diffHunk - Diff context
     */
    showCommentForm(path, line, diffHunk) {
        this.activeCommentForm = { path, line, diffHunk, isGlobal: false };
        this.render();
    }

    /**
     * Hide the comment form
     */
    hideCommentForm() {
        this.activeCommentForm = null;
        this.render();
    }

    /**
     * Set the count of remaining unviewed files
     * @param {number} count - Number of unviewed files
     */
    setRemainingUnviewed(count) {
        this.remainingUnviewed = count;
        // Update button without full re-render
        const btn = this.querySelector('.diff-next-button');
        if (btn) {
            if (count === 0) {
                btn.textContent = 'All files reviewed';
                btn.disabled = true;
                btn.classList.add('all-done');
            } else {
                btn.textContent = `Next file (${count} left)`;
                btn.disabled = false;
                btn.classList.remove('all-done');
            }
        }
    }

    render() {
        // Handle General selection - show only global comments
        if (this.selectedFile === '__general__') {
            this.innerHTML = this.renderGlobalComments();
            this.attachGlobalEventListeners();
            return;
        }

        if (!this.currentFile) {
            this.innerHTML = `
                <div class="empty-state">
                    <h3>Select a file</h3>
                    <p>Choose a file from the sidebar to view its diff.</p>
                </div>
            `;
            return;
        }

        const file = this.currentFile;
        const commentsByLine = groupCommentsByLine(this.currentComments);

        this.innerHTML = `
            <div class="diff-file" id="file-${escapeAttr(file.path)}">
                <div class="diff-file-header">
                    <span class="diff-file-status ${file.status}"></span>
                    <span>${escapeHtml(file.path)}</span>
                    ${file.old_path ? `<span style="color: var(--text-secondary)"> (renamed from ${escapeHtml(file.old_path)})</span>` : ''}
                </div>
                ${file.hunks.map((hunk, hunkIndex) => this.renderHunk(file, hunk, hunkIndex, commentsByLine)).join('')}
            </div>
            <button class="diff-next-button btn ${this.remainingUnviewed === 0 ? 'all-done' : ''}" ${this.remainingUnviewed === 0 ? 'disabled' : ''}>
                ${this.remainingUnviewed === 0 ? 'All files reviewed' : `Next file (${this.remainingUnviewed} left)`}
            </button>
        `;

        this.attachEventListeners(file);
    }

    renderHunk(file, hunk, hunkIndex, commentsByLine) {
        let html = `
            <div class="diff-hunk">
                <div class="diff-hunk-header">${escapeHtml(hunk.header)}</div>
                <table class="diff-table">
                    <colgroup>
                        <col class="diff-line-num-col">
                        <col class="diff-content-col">
                        <col class="diff-line-num-col">
                        <col class="diff-content-col">
                    </colgroup>
                    <tbody>
        `;

        for (const row of hunk.rows) {
            const rowType = getRowType(row);
            html += `<tr class="diff-row ${rowType}">`;

            // Old (left) side
            if (row.old_line) {
                const oldContent = row.old_line.highlighted_html || escapeHtml(row.old_line.content);
                html += `
                    <td class="diff-line-num">${row.old_line.number}</td>
                    <td class="diff-line-content old ${row.old_line.line_type}"><div class="diff-line-inner" data-side="old">${oldContent}</div></td>
                `;
            } else {
                html += `
                    <td class="diff-line-num"></td>
                    <td class="diff-line-content empty"></td>
                `;
            }

            // New (right) side - all lines are commentable
            if (row.new_line) {
                const newContent = row.new_line.highlighted_html || escapeHtml(row.new_line.content);
                html += `
                    <td class="diff-line-num">${row.new_line.number}</td>
                    <td class="diff-line-content new ${row.new_line.line_type} commentable"
                        data-path="${escapeAttr(file.path)}"
                        data-line="${row.new_line.number}"
                        data-hunk-index="${hunkIndex}"><div class="diff-line-inner" data-side="new">${newContent}</div></td>
                `;
            } else {
                html += `
                    <td class="diff-line-num"></td>
                    <td class="diff-line-content empty"></td>
                `;
            }

            html += `</tr>`;

            // Render comments for this line
            if (row.new_line) {
                const lineComments = commentsByLine[row.new_line.number] || [];
                if (lineComments.length > 0 ||
                    (this.activeCommentForm &&
                     this.activeCommentForm.path === file.path &&
                     this.activeCommentForm.line === row.new_line.number)) {
                    html += this.renderCommentThread(file, row.new_line.number, lineComments, hunk);
                }
            }
        }

        html += `
                    </tbody>
                </table>
            </div>
        `;

        return html;
    }

    renderGlobalComments() {
        let html = `
            <div class="global-comments-section">
                <div class="global-comments-header">General Comments</div>
                <div class="global-comments-list">
        `;

        for (const comment of this.globalComments) {
            html += `<review-comment data-comment-id="${escapeAttr(comment.id)}" data-global="true"></review-comment>`;
        }

        // always show the global comment form
        html += `
                    <comment-form data-path="__global__" data-global="true"></comment-form>
                </div>
            </div>
        `;

        return html;
    }

    renderCommentThread(file, line, comments, hunk) {
        const showForm = this.activeCommentForm &&
                        this.activeCommentForm.path === file.path &&
                        this.activeCommentForm.line === line;

        let html = `
            <tr class="comment-row">
                <td colspan="4">
                    <div class="comment-thread">
        `;

        for (const comment of comments) {
            html += `<review-comment data-comment-id="${escapeAttr(comment.id)}"></review-comment>`;
        }

        if (showForm) {
            html += `<comment-form data-path="${escapeAttr(file.path)}" data-line="${line}"></comment-form>`;
        }

        html += `
                    </div>
                </td>
            </tr>
        `;

        return html;
    }

    attachEventListeners(file) {
        // Click on commentable lines
        this.querySelectorAll('.diff-line-content.commentable').forEach(cell => {
            cell.addEventListener('click', () => {
                const path = cell.dataset.path;
                const line = parseInt(cell.dataset.line, 10);
                const hunkIndex = parseInt(cell.dataset.hunkIndex, 10);
                const hunk = file.hunks[hunkIndex];
                const diffHunk = hunk ? hunk.header : '';

                this.showCommentForm(path, line, diffHunk);
            });
        });

        // Initialize review-comment elements (both file-specific and global)
        this.querySelectorAll('review-comment').forEach(el => {
            const commentId = el.dataset.commentId;
            const isGlobal = el.dataset.global === 'true';

            if (isGlobal) {
                const comment = this.globalComments.find(c => c.id === commentId);
                if (comment) {
                    el.setComment(comment);
                }
            } else {
                const comment = this.currentComments.find(c => c.id === commentId);
                if (comment) {
                    el.setComment(comment);
                }
            }
        });

        // Initialize comment-form elements
        this.querySelectorAll('comment-form').forEach(el => {
            const path = el.dataset.path;
            const isGlobal = el.dataset.global === 'true';
            const line = isGlobal ? null : parseInt(el.dataset.line, 10);
            el.init(path, line, this.activeCommentForm?.diffHunk || '');
        });

        // Handle form cancel
        this.addEventListener('form-cancel', () => {
            this.hideCommentForm();
        });

        this.querySelectorAll(".diff-next-button").forEach(el => el.addEventListener("click", ev => {
            this.dispatchEvent(new CustomEvent('request-next-file', {
                bubbles: true,
            }));
        }));
    }

    attachGlobalEventListeners() {
        // Initialize review-comment elements for global comments
        this.querySelectorAll('review-comment').forEach(el => {
            const commentId = el.dataset.commentId;
            const comment = this.globalComments.find(c => c.id === commentId);
            if (comment) {
                el.setComment(comment);
            }
        });

        // Initialize comment-form elements
        this.querySelectorAll('comment-form').forEach(el => {
            el.init('__global__', null, '');
        });

        // Handle form cancel
        this.addEventListener('form-cancel', () => {
            this.hideCommentForm();
        });
    }
}

customElements.define('diff-view', DiffView);

export default DiffView;
