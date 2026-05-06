/**
 * <diff-view> custom element
 * Side-by-side diff view with comment support
 */

import './review-comment.js';
import './comment-form.js';
import { escapeHtml, escapeAttr, getRowType, groupCommentsByLine, getFilePath, getOldFilePath } from './utils.js';

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
        this.selectedFile = file ? getFilePath(file) : null;
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
        const filePath = getFilePath(file);
        const oldFilePath = getOldFilePath(file);
        const commentsByLine = groupCommentsByLine(this.currentComments);
        const fileComments = this.currentComments.filter(comment => comment.line === null);

        this.innerHTML = `
            <div class="diff-file" id="file-${escapeAttr(filePath)}">
                <div class="diff-file-header">
                    <div class="diff-file-title">
                        <span class="diff-file-status ${file.status}"></span>
                        <span>${escapeHtml(filePath)}</span>
                        ${oldFilePath ? `<span style="color: var(--text-secondary)"> (renamed from ${escapeHtml(oldFilePath)})</span>` : ''}
                    </div>
                    <button class="button button-small file-comment-button" data-path="${escapeAttr(filePath)}">Add file comment</button>
                </div>
                ${this.renderFileComments(filePath, fileComments)}
                ${file.chunks.map((chunk, chunkIndex) => this.renderChunk(file, chunk, chunkIndex, commentsByLine)).join('')}
            </div>
            <button class="diff-next-button btn ${this.remainingUnviewed === 0 ? 'all-done' : ''}" ${this.remainingUnviewed === 0 ? 'disabled' : ''}>
                ${this.remainingUnviewed === 0 ? 'All files reviewed' : `Next file (${this.remainingUnviewed} left)`}
            </button>
        `;

        this.attachEventListeners(file);
    }

    renderFileComments(filePath, comments) {
        const showForm = this.activeCommentForm &&
                        this.activeCommentForm.path === filePath &&
                        this.activeCommentForm.line === null;

        if (comments.length === 0 && !showForm) {
            return '';
        }

        let html = '<div class="file-comments-section">';
        for (const comment of comments) {
            html += `<review-comment data-comment-id="${escapeAttr(comment.id)}"></review-comment>`;
        }
        if (showForm) {
            html += `<comment-form data-path="${escapeAttr(filePath)}" data-file-comment="true"></comment-form>`;
        }
        html += '</div>';
        return html;
    }

    renderChunk(file, chunk, chunkIndex, commentsByLine) {
        const filePath = getFilePath(file);
        const hunkHeader = formatHunkHeader(chunk);
        let fromLine = chunk.from_range.start;
        let toLine = chunk.to_range.start;
        let html = `
            <div class="diff-hunk">
                <div class="diff-hunk-header">${escapeHtml(hunkHeader)}</div>
                <table class="diff-table">
                    <colgroup>
                        <col class="diff-line-num-col">
                        <col class="diff-content-col">
                        <col class="diff-line-num-col">
                        <col class="diff-content-col">
                    </colgroup>
                    <tbody>
        `;

        for (const line of chunk.lines) {
            const rowType = getRowType(line);
            const hasOld = line.status !== 'added';
            const hasNew = line.status !== 'removed';
            const oldNumber = hasOld ? fromLine : null;
            const newNumber = hasNew ? toLine : null;
            html += `<tr class="diff-row ${rowType}">`;

            // Old (left) side
            if (hasOld) {
                const oldContent = line.from_highlighted_html || escapeHtml(line.from);
                html += `
                    <td class="diff-line-num">${oldNumber}</td>
                    <td class="diff-line-content old ${rowType}"><div class="diff-line-inner" data-side="old">${oldContent}</div></td>
                `;
            } else {
                html += `
                    <td class="diff-line-num"></td>
                    <td class="diff-line-content empty"></td>
                `;
            }

            // New (right) side - all lines are commentable
            if (hasNew) {
                const newContent = line.to_highlighted_html || escapeHtml(line.to);
                const lineComments = commentsByLine[newNumber] || [];
                const hasComments = lineComments.length > 0;
                const commentIndicator = hasComments ? '<span class="line-comment-indicator"></span>' : '';
                html += `
                    <td class="diff-line-num ${hasComments ? 'has-comment' : ''}">${newNumber}${commentIndicator}</td>
                    <td class="diff-line-content new ${rowType} ${hasComments ? 'has-comment' : ''} commentable"
                        data-path="${escapeAttr(filePath)}"
                        data-line="${newNumber}"
                        data-chunk-index="${chunkIndex}"><div class="diff-line-inner" data-side="new">${newContent}</div></td>
                `;
            } else {
                html += `
                    <td class="diff-line-num"></td>
                    <td class="diff-line-content empty"></td>
                `;
            }

            html += `</tr>`;

            // Render comments for this line
            if (hasNew) {
                const lineComments = commentsByLine[newNumber] || [];
                if (lineComments.length > 0 ||
                    (this.activeCommentForm &&
                     this.activeCommentForm.path === filePath &&
                     this.activeCommentForm.line === newNumber)) {
                    html += this.renderCommentThread(file, newNumber, lineComments, chunk);
                }
            }

            if (hasOld) fromLine += 1;
            if (hasNew) toLine += 1;
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

    renderCommentThread(file, line, comments, chunk) {
        const filePath = getFilePath(file);
        const showForm = this.activeCommentForm &&
                        this.activeCommentForm.path === filePath &&
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
            html += `<comment-form data-path="${escapeAttr(filePath)}" data-line="${line}"></comment-form>`;
        }

        html += `
                    </div>
                </td>
            </tr>
        `;

        return html;
    }

    attachEventListeners(file) {
        this.querySelectorAll('.file-comment-button').forEach(button => {
            button.addEventListener('click', () => {
                this.showCommentForm(button.dataset.path, null, '');
            });
        });

        // Click on commentable lines
        this.querySelectorAll('.diff-line-content.commentable').forEach(cell => {
            cell.addEventListener('click', () => {
                const path = cell.dataset.path;
                const line = parseInt(cell.dataset.line, 10);
                const chunkIndex = parseInt(cell.dataset.chunkIndex, 10);
                const chunk = file.chunks[chunkIndex];
                const diffHunk = chunk ? chunkToUnified(chunk) : '';

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
            const line = isGlobal || el.dataset.fileComment === 'true'
                ? null
                : parseInt(el.dataset.line, 10);
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

    /**
     * Scroll to a specific comment and highlight it
     * @param {string} commentId - Comment ID to scroll to
     */
    scrollToComment(commentId) {
        const commentEl = this.querySelector(`review-comment[data-comment-id="${commentId}"]`);
        if (commentEl) {
            commentEl.scrollIntoView({ behavior: 'smooth', block: 'center' });
            commentEl.classList.add('nav-highlight');
            setTimeout(() => {
                commentEl.classList.remove('nav-highlight');
            }, 1500);
        }
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

function formatHunkHeader(chunk) {
    return `@@ -${chunk.from_range.start},${chunk.from_range.end - chunk.from_range.start} +${chunk.to_range.start},${chunk.to_range.end - chunk.to_range.start} @@`;
}

function chunkToUnified(chunk) {
    const lines = [formatHunkHeader(chunk)];
    for (const line of chunk.lines) {
        if (line.status === 'added') {
            lines.push(`+${line.to}`);
        } else if (line.status === 'removed') {
            lines.push(`-${line.from}`);
        } else {
            lines.push(` ${line.to}`);
        }
    }
    return lines.join('\n');
}

customElements.define('diff-view', DiffView);

export default DiffView;
