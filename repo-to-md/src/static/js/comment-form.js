/**
 * <comment-form> custom element
 * Inline form for adding new comments
 */

import { escapeHtml, setupAutoResizeTextarea, setupTextareaKeyboardShortcuts } from './utils.js';

class CommentForm extends HTMLElement {
    constructor() {
        super();
        this.path = '';
        this.line = 0;
        this.diffHunk = '';
        this.isSubmitting = false;
        this.draftBody = '';
        this.saveMessage = '';
        this.saveMessageType = '';
    }

    /**
     * Initialize the form with context
     * @param {string} path - File path
     * @param {number} line - Line number
     * @param {string} diffHunk - Diff context
     */
    init(path, line, diffHunk) {
        this.path = path;
        this.line = line;
        this.diffHunk = diffHunk;
        this.render();
    }

    connectedCallback() {
        this.render();
    }

    render() {
        this.innerHTML = `
            <textarea class="comment-form-textarea" placeholder="Write a comment..." ${this.isSubmitting ? 'disabled' : ''}>${escapeHtml(this.draftBody)}</textarea>
            <div class="comment-form-actions">
                <button class="button cancel-button" ${this.isSubmitting ? 'disabled' : ''}>Cancel</button>
                <button class="button button-primary submit-button" ${this.isSubmitting ? 'disabled' : ''}>${this.isSubmitting ? 'Saving...' : 'Add Comment'}</button>
            </div>
            <div class="comment-form-status ${this.saveMessageType}" role="${this.saveMessageType === 'error' ? 'alert' : 'status'}" aria-live="${this.saveMessageType === 'error' ? 'assertive' : 'polite'}">${escapeHtml(this.saveMessage)}</div>
        `;

        const textarea = this.querySelector('textarea');
        textarea.focus();

        this.querySelector('.cancel-button').addEventListener('click', () => {
            if (this.isSubmitting) return;
            this.dispatchEvent(new CustomEvent('form-cancel', { bubbles: true }));
        });

        this.querySelector('.submit-button').addEventListener('click', () => {
            this.submit();
        });

        setupAutoResizeTextarea(textarea);
        setupTextareaKeyboardShortcuts(
            textarea,
            () => this.submit(),
            () => {
                if (!this.isSubmitting) this.dispatchEvent(new CustomEvent('form-cancel', { bubbles: true }));
            }
        );
    }

    setSaveError(message) {
        this.isSubmitting = false;
        this.saveMessage = message;
        this.saveMessageType = 'error';
        this.render();
        this.querySelector('textarea')?.focus();
    }

    submit() {
        const textarea = this.querySelector('textarea');
        if (this.isSubmitting) return;
        const body = textarea.value.trim();

        if (!body) {
            this.saveMessage = 'Enter a comment before saving.';
            this.saveMessageType = 'error';
            this.render();
            this.querySelector('textarea')?.focus();
            return;
        }

        this.draftBody = body;
        this.isSubmitting = true;
        this.saveMessage = 'Saving comment...';
        this.saveMessageType = 'pending';
        this.render();

        this.dispatchEvent(new CustomEvent('comment-submit', {
            detail: {
                path: this.path,
                line: this.line,
                body: body,
                diff_hunk: this.diffHunk,
            },
            bubbles: true,
        }));
    }
}

customElements.define('comment-form', CommentForm);

export default CommentForm;
