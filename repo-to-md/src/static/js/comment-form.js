/**
 * <comment-form> custom element
 * Inline form for adding new comments
 */

import { setupAutoResizeTextarea, setupTextareaKeyboardShortcuts } from './utils.js';

class CommentForm extends HTMLElement {
    constructor() {
        super();
        this.path = '';
        this.line = 0;
        this.diffHunk = '';
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
            <textarea class="comment-form-textarea" placeholder="Write a comment..."></textarea>
            <div class="comment-form-actions">
                <button class="button cancel-button">Cancel</button>
                <button class="button button-primary submit-button">Add Comment</button>
            </div>
        `;

        const textarea = this.querySelector('textarea');
        textarea.focus();

        this.querySelector('.cancel-button').addEventListener('click', () => {
            this.dispatchEvent(new CustomEvent('form-cancel', { bubbles: true }));
        });

        this.querySelector('.submit-button').addEventListener('click', () => {
            this.submit();
        });

        setupAutoResizeTextarea(textarea);
        setupTextareaKeyboardShortcuts(
            textarea,
            () => this.submit(),
            () => this.dispatchEvent(new CustomEvent('form-cancel', { bubbles: true }))
        );
    }

    submit() {
        const textarea = this.querySelector('textarea');
        const body = textarea.value.trim();

        if (!body) {
            return;
        }

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
