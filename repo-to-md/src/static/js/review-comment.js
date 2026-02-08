/**
 * <review-comment> custom element
 * Displays a single review comment with edit/delete actions
 */

import { escapeHtml, setupAutoResizeTextarea, setupTextareaKeyboardShortcuts } from './utils.js';

class ReviewComment extends HTMLElement {
    constructor() {
        super();
        this.comment = null;
        this.isEditing = false;
    }

    /**
     * Set the comment data
     * @param {Object} comment - The comment object
     */
    setComment(comment) {
        this.comment = comment;
        this.render();
    }

    render() {
        if (!this.comment) {
            this.innerHTML = '';
            return;
        }

        if (this.isEditing) {
            this.renderEditMode();
        } else {
            this.renderViewMode();
        }
    }

    renderViewMode() {
        this.innerHTML = `
            <div class="comment-header">
                <span class="comment-author">${escapeHtml(this.comment.user.login)}</span>
                <div class="comment-actions">
                    <button class="button button-small edit-button">Edit</button>
                    <button class="button button-small button-danger delete-button">Delete</button>
                </div>
            </div>
            <div class="comment-body">${escapeHtml(this.comment.body)}</div>
        `;

        this.querySelector('.edit-button').addEventListener('click', () => {
            this.isEditing = true;
            this.render();
        });

        this.querySelector('.delete-button').addEventListener('click', () => {
            this.dispatchEvent(new CustomEvent('comment-delete', {
                detail: { id: this.comment.id },
                bubbles: true,
            }));
        });
    }

    renderEditMode() {
        this.innerHTML = `
            <textarea class="comment-form-textarea">${escapeHtml(this.comment.body)}</textarea>
            <div class="comment-form-actions">
                <button class="button cancel-button">Cancel</button>
                <button class="button button-primary save-button">Save</button>
            </div>
        `;

        const textarea = this.querySelector('textarea');
        textarea.focus();
        textarea.setSelectionRange(textarea.value.length, textarea.value.length);

        setupAutoResizeTextarea(textarea);

        const cancelEdit = () => {
            this.isEditing = false;
            this.render();
        };

        const saveEdit = () => {
            const newBody = textarea.value.trim();
            if (newBody && newBody !== this.comment.body) {
                this.dispatchEvent(new CustomEvent('comment-update', {
                    detail: { id: this.comment.id, body: newBody },
                    bubbles: true,
                }));
            }
            this.isEditing = false;
            this.render();
        };

        this.querySelector('.cancel-button').addEventListener('click', cancelEdit);
        this.querySelector('.save-button').addEventListener('click', saveEdit);
        setupTextareaKeyboardShortcuts(textarea, saveEdit, cancelEdit);
    }
}

customElements.define('review-comment', ReviewComment);

export default ReviewComment;
