/**
 * Main application module
 * Coordinates the diff view, file tree, and comments
 */

import * as api from './api.js';
import './file-tree.js';
import './diff-view.js';
import { getCommentsByFile, computeFileTreeItems, getCommentPositions, getFilePath } from './utils.js';

export class App {
    constructor() {
        this.diff = null;
        this.comments = [];
        this.viewedFiles = [];
        this.filesMap = new Map();  // path -> file object
        this.username = 'user';
        this.lastNavigatedCommentId = null;

        this.fileTree = document.querySelector('file-tree');
        this.diffView = document.querySelector('diff-view');
        this.refInfo = document.getElementById('refInfo');
        this.shutdownBtn = document.getElementById('shutdownBtn');
        this.prevCommentBtn = document.getElementById('prevCommentBtn');
        this.nextCommentBtn = document.getElementById('nextCommentBtn');

        this.setupEventListeners();
    }

    async init() {
        try {
            // Load all session data in a single request
            const session = await api.fetchSession();

            this.diff = session;
            this.comments = session.comments;
            this.viewedFiles = session.viewed_files;
            this.filesMap = new Map(session.files.map(f => [getFilePath(f), f]));

            // Update ref info
            const refText = session.end_ref
                ? `${session.start_ref}..${session.end_ref}`
                : `${session.start_ref}`;
            this.refInfo.textContent = refText;

            // Update file tree from session data
            this.updateFileTree();

            // Update diff view with global comments
            const commentsByFile = getCommentsByFile(this.comments);
            this.diffView.setGlobalComments(commentsByFile['__general__'] || []);

            // Select first file if available
            if (session.files.length > 0) {
                const firstPath = getFilePath(session.files[0]);
                this.selectFileForDiffView(firstPath);
                this.fileTree.selectFile(firstPath);
            }
        } catch (error) {
            console.error('Failed to load data:', error);
            this.refInfo.textContent = 'Error loading diff';
        }
    }

    /**
     * Update file tree UI from current session
     */
    updateFileTree() {
        const commentsByFile = getCommentsByFile(this.comments);
        const viewedSet = new Set(this.viewedFiles);
        const items = computeFileTreeItems(
            Array.from(this.filesMap.values()),
            commentsByFile,
            viewedSet
        );
        const generalCount = (commentsByFile['__general__'] || []).filter(c => !c.is_minimized).length;
        this.fileTree.setItems(items, generalCount);
        this.updateRemainingUnviewed();
    }

    /**
     * Update the remaining unviewed count in the diff view
     */
    updateRemainingUnviewed() {
        const viewedSet = new Set(this.viewedFiles);
        const unviewedCount = Array.from(this.filesMap.keys()).filter(p => !viewedSet.has(p)).length;
        this.diffView.setRemainingUnviewed(unviewedCount);
    }

    /**
     * Select a file to display in the diff view
     * @param {string} path - File path or '__general__'
     */
    selectFileForDiffView(path) {
        const commentsByFile = getCommentsByFile(this.comments);
        if (path === '__general__') {
            this.diffView.setCurrentFile(null, []);
            this.diffView.setGlobalComments(commentsByFile['__general__'] || []);
            this.diffView.selectedFile = '__general__';
            this.diffView.render();
        } else {
            const file = this.filesMap.get(path);
            const comments = commentsByFile[path] || [];
            this.diffView.setCurrentFile(file, comments);
        }
    }

    setupEventListeners() {
        this.fileTree.addEventListener('file-selected', (e) => {
            this.selectFileForDiffView(e.detail.path);
        });

        // Viewed file toggle
        this.fileTree.addEventListener('viewed-toggle', async (e) => {
            await this.toggleViewedFile(e.detail.path, e.detail.viewed);
        });

        // Comment submission
        document.addEventListener('comment-submit', async (e) => {
            await this.createComment(e.detail);
        });

        // Comment update
        document.addEventListener('comment-update', async (e) => {
            await this.updateComment(e.detail.id, e.detail.body);
        });

        // Comment deletion
        document.addEventListener('comment-delete', async (e) => {
            await this.deleteComment(e.detail.id);
        });

        // Comment minimize toggle
        document.addEventListener('comment-minimize', async (e) => {
            await this.toggleMinimizeComment(e.detail.id);
        });

        // Comment navigation buttons in header
        this.prevCommentBtn.addEventListener('click', () => {
            this.navigateComment(-1);
        });

        this.nextCommentBtn.addEventListener('click', () => {
            this.navigateComment(1);
        });

        // Sync file tree when diff view requests a file selection (e.g., for global comments)
        document.addEventListener('request-file-select', (e) => {
            this.fileTree.selectFile(e.detail.path);
        });
        document.addEventListener("request-next-file", () => {
            const currentPath = this.fileTree.selectedFile;
            if (!currentPath) return;

            this.toggleViewedFile(currentPath, true);
            this.fileTree.selectNextUnviewed();
        });

        // Shutdown button
        this.shutdownBtn.addEventListener('click', () => this.handleShutdown());
    }

    async handleShutdown() {
        if (!confirm('Quit the review? You can restart with the same command.')) {
            return;
        }

        try {
            await api.shutdown();
            // Close the browser window/tab
            window.close();
        } catch (error) {
            // Server might have already shut down, try to close anyway
            window.close();
        }
    }

    navigateFile(direction) {
        if (!this.diff?.files?.length) return;

        const navList = ['__general__', ...this.diff.files.map(getFilePath)];
        const currentIndex = navList.indexOf(this.fileTree.selectedFile);
        const baseIndex = currentIndex === -1 ? (direction > 0 ? -1 : 0) : currentIndex;
        const newIndex = (baseIndex + direction + navList.length) % navList.length;

        this.fileTree.selectFile(navList[newIndex]);
    }

    toggleCurrentFileViewed() {
        const currentPath = this.fileTree.selectedFile;
        if (!currentPath) return;

        const isViewed = this.viewedFiles.includes(currentPath);
        this.toggleViewedFile(currentPath, !isViewed);
    }

    async toggleViewedFile(path, viewed) {
        try {
            await api.setFileViewed(path, viewed);
            if (viewed) {
                this.viewedFiles.push(path);
            } else {
                this.viewedFiles = this.viewedFiles.filter(p => p !== path);
            }
            // Update just that item instead of full refresh
            this.fileTree.updateItem(path, { isViewed: viewed });
            this.updateRemainingUnviewed();
        } catch (error) {
            console.error('Failed to toggle viewed status:', error);
            alert('Failed to update viewed status. Please try again.');
        }
    }

    async createComment(data) {
        try {
            const result = await api.createComment({
                path: data.path,
                line: data.line,
                body: data.body,
                user: this.username,
                diff_hunk: data.diff_hunk,
            });

            this.comments.push(result.comment);
            this.diffView.hideCommentForm();
            this.updateViews();
        } catch (error) {
            console.error('Failed to create comment:', error);
            alert('Failed to create comment. Please try again.');
        }
    }

    async updateComment(id, body) {
        try {
            const result = await api.updateComment(id, body);
            const index = this.comments.findIndex(c => c.id === id);
            if (index !== -1) {
                this.comments[index] = result.comment;
            }
            this.updateViews();
        } catch (error) {
            console.error('Failed to update comment:', error);
            alert('Failed to update comment. Please try again.');
        }
    }

    async deleteComment(id) {
        if (!confirm('Are you sure you want to delete this comment?')) {
            return;
        }

        try {
            await api.deleteComment(id);
            this.comments = this.comments.filter(c => c.id !== id);
            this.updateViews();
        } catch (error) {
            console.error('Failed to delete comment:', error);
            alert('Failed to delete comment. Please try again.');
        }
    }

    async toggleMinimizeComment(id) {
        try {
            const result = await api.toggleMinimizeComment(id);
            const index = this.comments.findIndex(c => c.id === id);
            if (index !== -1) {
                this.comments[index] = result.comment;
            }
            this.updateViews();
        } catch (error) {
            console.error('Failed to toggle minimize comment:', error);
            alert('Failed to toggle minimize. Please try again.');
        }
    }

    navigateComment(direction) {
        const commentsByFile = getCommentsByFile(this.comments);
        const positions = getCommentPositions(
            Array.from(this.filesMap.values()),
            commentsByFile,
            true
        );

        if (positions.length === 0) return;

        // Find current index
        let currentIndex = -1;
        if (this.lastNavigatedCommentId) {
            currentIndex = positions.findIndex(p => p.id === this.lastNavigatedCommentId);
        }

        // If no previous navigation or not found, use first/last comment in current file
        if (currentIndex === -1) {
            const currentPath = this.diffView.selectedFile;
            const filePositions = positions.filter(p => p.path === currentPath);
            if (filePositions.length > 0) {
                currentIndex = positions.indexOf(direction > 0 ? filePositions[0] : filePositions[filePositions.length - 1]);
            } else {
                currentIndex = direction > 0 ? -1 : positions.length;
            }
        }

        // Calculate target index with wrapping
        const targetIndex = (currentIndex + direction + positions.length) % positions.length;
        const target = positions[targetIndex];

        // Navigate to target file if different
        if (target.path !== this.diffView.selectedFile) {
            this.fileTree.selectFile(target.path);
            // Scroll to comment after file loads
            requestAnimationFrame(() => {
                this.diffView.scrollToComment(target.id);
            });
        } else {
            this.diffView.scrollToComment(target.id);
        }

        this.lastNavigatedCommentId = target.id;
    }

    updateViews() {
        // Update file tree with new comment counts
        this.updateFileTree();

        // Update only current file's comments in DiffView
        const currentPath = this.diffView.selectedFile;
        const commentsByFile = getCommentsByFile(this.comments);
        if (currentPath === '__general__') {
            this.diffView.setGlobalComments(commentsByFile['__general__'] || []);
        } else if (currentPath) {
            this.diffView.updateCurrentComments(commentsByFile[currentPath] || []);
        }
    }
}
