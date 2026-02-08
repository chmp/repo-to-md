/**
 * <file-tree> custom element
 * Displays the list of changed files with status icons and comment counts
 */

import { escapeHtml, getFileName } from './utils.js';

const FILE_ICONS = {
    added: `<svg class="file-tree-icon added" viewBox="0 0 16 16" fill="currentColor">
        <path d="M8 4a.5.5 0 01.5.5v3h3a.5.5 0 010 1h-3v3a.5.5 0 01-1 0v-3h-3a.5.5 0 010-1h3v-3A.5.5 0 018 4z"/>
    </svg>`,
    deleted: `<svg class="file-tree-icon deleted" viewBox="0 0 16 16" fill="currentColor">
        <path d="M4.5 8a.5.5 0 01.5-.5h6a.5.5 0 010 1H5a.5.5 0 01-.5-.5z"/>
    </svg>`,
    modified: `<svg class="file-tree-icon modified" viewBox="0 0 16 16" fill="currentColor">
        <path d="M8 4a4 4 0 100 8 4 4 0 000-8z"/>
    </svg>`,
    renamed: `<svg class="file-tree-icon renamed" viewBox="0 0 16 16" fill="currentColor">
        <path d="M1 8a.5.5 0 01.5-.5h11.793l-3.147-3.146a.5.5 0 01.708-.708l4 4a.5.5 0 010 .708l-4 4a.5.5 0 01-.708-.708L13.293 8.5H1.5A.5.5 0 011 8z"/>
    </svg>`,
};

const VIEWED_ICON = `<svg class="viewed-icon" viewBox="0 0 16 16" fill="currentColor">
    <path fill-rule="evenodd" d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/>
</svg>`;

const UNVIEWED_ICON = `<svg class="unviewed-icon" viewBox="0 0 16 16" fill="currentColor">
    <path fill-rule="evenodd" d="M8 1.5a6.5 6.5 0 100 13 6.5 6.5 0 000-13zM0 8a8 8 0 1116 0A8 8 0 010 8z"/>
</svg>`;

const COMMENT_ICON = `<svg class="file-tree-icon comment" viewBox="0 0 16 16" fill="currentColor">
    <path d="M1 2.75C1 1.784 1.784 1 2.75 1h10.5c.966 0 1.75.784 1.75 1.75v7.5A1.75 1.75 0 0113.25 12H9.06l-2.573 2.573A1.458 1.458 0 014 13.543V12H2.75A1.75 1.75 0 011 10.25v-7.5zm1.75-.25a.25.25 0 00-.25.25v7.5c0 .138.112.25.25.25h2a.75.75 0 01.75.75v2.19l2.72-2.72a.75.75 0 01.53-.22h4.5a.25.25 0 00.25-.25v-7.5a.25.25 0 00-.25-.25H2.75z"/>
</svg>`;

class FileTree extends HTMLElement {
    constructor() {
        super();
        this.items = [];  // Array of { path, status, commentCount, isViewed }
        this.generalCommentCount = 0;
        this.selectedFile = null;
    }

    connectedCallback() {
        this.render();
    }

    /**
     * Set the file tree items
     * @param {Array} items - Array of { path, status, commentCount, isViewed }
     * @param {number} generalCommentCount - Number of general comments
     */
    setItems(items, generalCommentCount) {
        this.items = items;
        this.generalCommentCount = generalCommentCount;
        this.render();
    }

    /**
     * Update a single item's properties
     * @param {string} path - File path to update
     * @param {Object} updates - Properties to update (e.g., { isViewed: true })
     */
    updateItem(path, updates) {
        const item = this.items.find(i => i.path === path);
        if (item) {
            Object.assign(item, updates);
            this.render();
        }
    }

    /**
     * Select a file
     * @param {string} path - File path to select
     */
    selectFile(path) {
        this.selectedFile = path;
        this.render();
        // Scroll selected item into view
        const activeItem = this.querySelector('.file-tree-item.active');
        if (activeItem) {
            activeItem.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
        }
        this.dispatchEvent(new CustomEvent('file-selected', {
            detail: { path },
            bubbles: true,
        }));
    }

    render() {
        this.innerHTML = `
            <div class="file-tree-header">
                <span>Changed Files (${this.items.length})</span>
            </div>
            <ul class="file-tree-list">
                    <li class="file-tree-item file-tree-general ${this.selectedFile === '__general__' ? 'active' : ''}"
                        data-path="__general__">
                        ${COMMENT_ICON}
                        <span class="file-tree-name">General</span>
                        ${this.generalCommentCount > 0 ? `
                            <span class="file-tree-badge">${this.generalCommentCount}</span>
                        ` : ''}
                    </li>
                ${this.items.map(item => `
                    <li class="file-tree-item ${this.selectedFile === item.path ? 'active' : ''} ${item.isViewed ? 'viewed' : ''}"
                        data-path="${escapeHtml(item.path)}">
                        <button class="viewed-toggle" data-path="${escapeHtml(item.path)}" title="${item.isViewed ? 'Mark as unviewed' : 'Mark as viewed'}">
                            ${item.isViewed ? VIEWED_ICON : UNVIEWED_ICON}
                        </button>
                        ${FILE_ICONS[item.status] || FILE_ICONS.modified}
                        <span class="file-tree-name" title="${escapeHtml(item.path)}">
                            ${escapeHtml(getFileName(item.path))}
                        </span>
                        ${item.commentCount > 0 ? `
                            <span class="file-tree-badge">${item.commentCount}</span>
                        ` : ''}
                    </li>
                `).join('')}
            </ul>
        `;

        // Add click handlers for file selection
        this.querySelectorAll('.file-tree-item').forEach(el => {
            el.addEventListener('click', (e) => {
                // Don't select if clicking the viewed toggle
                if (e.target.closest('.viewed-toggle')) return;
                this.selectFile(el.dataset.path);
            });
        });

        // Add click handlers for viewed toggles
        this.querySelectorAll('.viewed-toggle').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const path = btn.dataset.path;
                const item = this.items.find(i => i.path === path);
                const isCurrentlyViewed = item ? item.isViewed : false;
                this.dispatchEvent(new CustomEvent('viewed-toggle', {
                    detail: { path, viewed: !isCurrentlyViewed },
                    bubbles: true,
                }));
            });
        });

        // Add click handler for next unviewed button
        const nextBtn = this.querySelector('.next-unviewed-button');
        if (nextBtn) {
            nextBtn.addEventListener('click', () => {
                this.selectNextUnviewed();
            });
        }
    }

    selectNextUnviewed() {
        const unviewedItems = this.items.filter(i => !i.isViewed);
        if (unviewedItems.length === 0) return;

        // Find the current index
        const currentIndex = this.items.findIndex(i => i.path === this.selectedFile);

        // Find next unviewed after current position
        for (let i = currentIndex + 1; i < this.items.length; i++) {
            if (!this.items[i].isViewed) {
                this.selectFile(this.items[i].path);
                return;
            }
        }

        // Wrap around to beginning
        for (let i = 0; i <= currentIndex; i++) {
            if (!this.items[i].isViewed) {
                this.selectFile(this.items[i].path);
                return;
            }
        }
    }
}

customElements.define('file-tree', FileTree);

export default FileTree;
