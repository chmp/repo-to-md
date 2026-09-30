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
        this.pendingMutations = new Set();
        this.statusTimer = null;

        this.fileTree = document.querySelector('file-tree');
        this.diffView = document.querySelector('diff-view');
        this.refInfo = document.getElementById('refInfo');
        this.appStatus = document.getElementById('appStatus');
        this.commentNavStatus = document.getElementById('commentNavStatus');
        this.shutdownBtn = document.getElementById('shutdownBtn');
        this.prevCommentBtn = document.getElementById('prevCommentBtn');
        this.nextCommentBtn = document.getElementById('nextCommentBtn');

        this.setupEventListeners();
    }

    async init() {
        this.showLoading();

        try {
            // Load all session data in a single request
            const session = await api.fetchSession();

            this.diff = session;
            this.comments = session.comments;
            this.viewedFiles = session.viewed_files;
            this.filesMap = new Map(session.files.map(f => [getFilePath(f), f]));
            this.updateCommentNavigation();

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
            } else {
                this.diffView.setCurrentFile(null, []);
            }
        } catch (error) {
            console.error('Failed to load data:', error);
            this.showLoadError();
        }
    }

    showLoading() {
        this.refInfo.textContent = 'Loading diff...';
        this.fileTree.innerHTML = `
            <div class="file-tree-header">
                <span>Changed Files</span>
            </div>
            <div class="file-tree-loading">Loading files...</div>
        `;
        this.diffView.innerHTML = `
            <div class="empty-state">
                <h3>Loading diff</h3>
                <p>Fetching the diff and comments.</p>
            </div>
        `;
    }

    showLoadError() {
        this.refInfo.textContent = 'Error loading diff';
        this.updateCommentNavigation();
        this.diffView.innerHTML = `
            <div class="empty-state">
                <h3>Error loading diff</h3>
                <p>Check the server logs and refresh this page.</p>
            </div>
        `;
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
            await this.toggleViewedFile(e.detail.path, e.detail.viewed, e.detail.button);
        });

        // Comment submission
        document.addEventListener('comment-submit', async (e) => {
            await this.createComment(e.detail, e.target);
        });

        // Comment update
        document.addEventListener('comment-update', async (e) => {
            await this.updateComment(e.detail.id, e.detail.body, e.target);
        });

        // Comment deletion
        document.addEventListener('comment-delete', async (e) => {
            await this.deleteComment(e.detail.id, e.target);
        });

        // Comment minimize toggle
        document.addEventListener('comment-minimize', async (e) => {
            await this.toggleMinimizeComment(e.detail.id, e.target);
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

    async toggleViewedFile(path, viewed, button = null) {
        const operation = `viewed:${path}`;
        if (this.pendingMutations.has(operation)) return;
        this.pendingMutations.add(operation);
        if (button) button.disabled = true;
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
            this.showStatus(viewed ? 'File marked as viewed.' : 'File marked as unviewed.', 'success');
        } catch (error) {
            console.error('Failed to toggle viewed status:', error);
            this.showStatus(`Could not update viewed status: ${this.getErrorMessage(error)}`, 'error');
        } finally {
            this.pendingMutations.delete(operation);
            if (button?.isConnected) button.disabled = false;
        }
    }

    async createComment(data, form) {
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
            this.showStatus('Comment added.', 'success');
        } catch (error) {
            console.error('Failed to create comment:', error);
            const message = `Could not add comment: ${this.getErrorMessage(error)}`;
            form?.setSaveError(message);
            this.showStatus(message, 'error');
        }
    }

    async updateComment(id, body, commentElement) {
        try {
            const result = await api.updateComment(id, body);
            const index = this.comments.findIndex(c => c.id === id);
            if (index !== -1) {
                this.comments[index] = result.comment;
            }
            this.updateViews();
            this.showStatus('Comment saved.', 'success');
        } catch (error) {
            console.error('Failed to update comment:', error);
            const message = `Could not save comment: ${this.getErrorMessage(error)}`;
            commentElement?.setSaveError(message);
            this.showStatus(message, 'error');
        }
    }

    async deleteComment(id, commentElement) {
        if (!confirm('Are you sure you want to delete this comment?')) {
            return;
        }

        const operation = `delete:${id}`;
        if (this.pendingMutations.has(operation)) return;
        this.pendingMutations.add(operation);
        const button = commentElement?.querySelector('.delete-button');
        if (button) button.disabled = true;
        try {
            await api.deleteComment(id);
            this.comments = this.comments.filter(c => c.id !== id);
            this.updateViews();
            this.showStatus('Comment deleted.', 'success');
        } catch (error) {
            console.error('Failed to delete comment:', error);
            this.showStatus(`Could not delete comment: ${this.getErrorMessage(error)}`, 'error');
        } finally {
            this.pendingMutations.delete(operation);
            if (button?.isConnected) button.disabled = false;
        }
    }

    async toggleMinimizeComment(id, commentElement) {
        const operation = `resolve:${id}`;
        if (this.pendingMutations.has(operation)) return;
        this.pendingMutations.add(operation);
        const button = commentElement?.querySelector('.minimize-button');
        if (button) button.disabled = true;
        try {
            const result = await api.toggleMinimizeComment(id);
            const index = this.comments.findIndex(c => c.id === id);
            if (index !== -1) {
                this.comments[index] = result.comment;
            }
            this.updateViews();
            this.showStatus(result.comment.is_minimized ? 'Comment resolved.' : 'Comment reopened.', 'success');
        } catch (error) {
            console.error('Failed to toggle minimize comment:', error);
            this.showStatus(`Could not update comment status: ${this.getErrorMessage(error)}`, 'error');
        } finally {
            this.pendingMutations.delete(operation);
            if (button?.isConnected) button.disabled = false;
        }
    }

    navigateComment(direction) {
        const commentsByFile = getCommentsByFile(this.comments);
        const positions = getCommentPositions(
            Array.from(this.filesMap.values()),
            commentsByFile,
            true
        );

        if (positions.length === 0) {
            this.updateCommentNavigation();
            return;
        }

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
        this.updateCommentNavigation();
    }

    updateCommentNavigation() {
        const commentsByFile = getCommentsByFile(this.comments);
        const positions = getCommentPositions(Array.from(this.filesMap.values()), commentsByFile, true);
        const count = positions.length;

        this.prevCommentBtn.disabled = count === 0;
        this.nextCommentBtn.disabled = count === 0;
        if (count === 0) {
            this.lastNavigatedCommentId = null;
            this.commentNavStatus.textContent = 'No comments';
            return;
        }

        const currentIndex = positions.findIndex(position => position.id === this.lastNavigatedCommentId);
        if (currentIndex === -1) {
            this.lastNavigatedCommentId = null;
            this.commentNavStatus.textContent = `${count} ${count === 1 ? 'comment' : 'comments'}`;
            return;
        }
        this.commentNavStatus.textContent = `Comment ${currentIndex + 1} of ${count}`;
    }

    showStatus(message, type = 'info') {
        if (!this.appStatus) return;
        clearTimeout(this.statusTimer);
        this.appStatus.hidden = false;
        this.appStatus.className = `app-status ${type}`;
        this.appStatus.setAttribute('role', type === 'error' ? 'alert' : 'status');
        this.appStatus.setAttribute('aria-live', type === 'error' ? 'assertive' : 'polite');
        this.appStatus.textContent = message;
        this.statusTimer = setTimeout(() => {
            this.appStatus.hidden = true;
        }, type === 'error' ? 8000 : 4000);
    }

    getErrorMessage(error) {
        return error instanceof Error && error.message ? error.message : 'Please try again.';
    }

    updateViews() {
        // Update file tree with new comment counts
        this.updateFileTree();

        // Update only current file's comments in DiffView
        const currentPath = this.diffView.selectedFile;
        const commentsByFile = getCommentsByFile(this.comments);
        this.updateCommentNavigation();
        if (currentPath === '__general__') {
            this.diffView.setGlobalComments(commentsByFile['__general__'] || []);
        } else if (currentPath) {
            this.diffView.updateCurrentComments(commentsByFile[currentPath] || []);
        }
    }
}
