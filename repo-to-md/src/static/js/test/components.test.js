import '../comment-form.js';
import '../review-comment.js';

// Helper to create and cleanup test elements
function withElement(tagName, callback) {
    const el = document.createElement(tagName);
    document.body.appendChild(el);
    try {
        return callback(el);
    } finally {
        el.remove();
    }
}

// CommentForm tests
minitest("CommentForm", ({ run, assertEqual }) => {
    run("submit does nothing when textarea is empty", ({ assertEqual }) => {
        withElement('comment-form', (form) => {
            form.init('test.js', 10, 'diff context');

            let eventFired = false;
            form.addEventListener('comment-submit', () => {
                eventFired = true;
            });

            form.submit();

            assertEqual(eventFired, false);
        });
    });

    run("submit does nothing when textarea is whitespace only", ({ assertEqual }) => {
        withElement('comment-form', (form) => {
            form.init('test.js', 10, 'diff context');
            form.querySelector('textarea').value = '   \n\t  ';

            let eventFired = false;
            form.addEventListener('comment-submit', () => {
                eventFired = true;
            });

            form.submit();

            assertEqual(eventFired, false);
        });
    });

    run("submit emits comment-submit event with correct detail", ({ assertEqual }) => {
        withElement('comment-form', (form) => {
            form.init('src/app.js', 42, '@@ -1,5 +1,6 @@');
            form.querySelector('textarea').value = 'This is my comment';

            let detail = null;
            form.addEventListener('comment-submit', (e) => {
                detail = e.detail;
            });

            form.submit();

            assertEqual(detail.path, 'src/app.js');
            assertEqual(detail.line, 42);
            assertEqual(detail.body, 'This is my comment');
            assertEqual(detail.diff_hunk, '@@ -1,5 +1,6 @@');
        });
    });

    run("cancel button emits form-cancel event", ({ assertEqual }) => {
        withElement('comment-form', (form) => {
            form.init('test.js', 10, '');

            let eventFired = false;
            form.addEventListener('form-cancel', () => {
                eventFired = true;
            });

            form.querySelector('.cancel-button').click();

            assertEqual(eventFired, true);
        });
    });

    run("Escape key emits form-cancel event", ({ assertEqual }) => {
        withElement('comment-form', (form) => {
            form.init('test.js', 10, '');

            let eventFired = false;
            form.addEventListener('form-cancel', () => {
                eventFired = true;
            });

            const textarea = form.querySelector('textarea');
            const event = new KeyboardEvent('keydown', { key: 'Escape' });
            textarea.dispatchEvent(event);

            assertEqual(eventFired, true);
        });
    });

    run("Ctrl+Enter triggers submit", ({ assertEqual }) => {
        withElement('comment-form', (form) => {
            form.init('test.js', 10, '');
            form.querySelector('textarea').value = 'test comment';

            let detail = null;
            form.addEventListener('comment-submit', (e) => {
                detail = e.detail;
            });

            const textarea = form.querySelector('textarea');
            const event = new KeyboardEvent('keydown', {
                key: 'Enter',
                ctrlKey: true,
            });
            textarea.dispatchEvent(event);

            assertEqual(detail !== null, true);
            assertEqual(detail.body, 'test comment');
        });
    });

    run("handles global comments (null line)", ({ assertEqual }) => {
        withElement('comment-form', (form) => {
            form.init('__global__', null, '');
            form.querySelector('textarea').value = 'global comment';

            let detail = null;
            form.addEventListener('comment-submit', (e) => {
                detail = e.detail;
            });

            form.submit();

            assertEqual(detail.path, '__global__');
            assertEqual(detail.line, null);
        });
    });
});

// ReviewComment tests
minitest("ReviewComment", ({ run, assertEqual }) => {
    run("renders comment in view mode", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-1',
                user: { login: 'testuser' },
                body: 'This is a test comment',
            });

            const author = el.querySelector('.comment-author');
            const body = el.querySelector('.comment-body');

            assertEqual(author.textContent, 'testuser');
            assertEqual(body.textContent, 'This is a test comment');
        });
    });

    run("edit button switches to edit mode", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-1',
                user: { login: 'testuser' },
                body: 'Original comment',
            });

            // Click edit button
            el.querySelector('.edit-button').click();

            // Should now have a textarea
            const textarea = el.querySelector('textarea');
            assertEqual(textarea !== null, true);
            assertEqual(textarea.value, 'Original comment');
        });
    });

    run("cancel button in edit mode returns to view mode", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-1',
                user: { login: 'testuser' },
                body: 'Original comment',
            });

            // Enter edit mode
            el.querySelector('.edit-button').click();

            // Click cancel
            el.querySelector('.cancel-button').click();

            // Should be back in view mode
            const body = el.querySelector('.comment-body');
            assertEqual(body !== null, true);
            assertEqual(body.textContent, 'Original comment');
        });
    });

    run("save button emits comment-update only if body changed", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-1',
                user: { login: 'testuser' },
                body: 'Original comment',
            });

            el.querySelector('.edit-button').click();

            let detail = null;
            el.addEventListener('comment-update', (e) => {
                detail = e.detail;
            });

            // Don't change the text, just save
            el.querySelector('.save-button').click();

            // Should not emit event when body unchanged
            assertEqual(detail, null);
        });
    });

    run("save button emits comment-update when body changed", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-1',
                user: { login: 'testuser' },
                body: 'Original comment',
            });

            el.querySelector('.edit-button').click();

            let detail = null;
            el.addEventListener('comment-update', (e) => {
                detail = e.detail;
            });

            // Change the text
            el.querySelector('textarea').value = 'Updated comment';
            el.querySelector('.save-button').click();

            assertEqual(detail.id, 'comment-1');
            assertEqual(detail.body, 'Updated comment');
        });
    });

    run("delete button emits comment-delete with comment id", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-123',
                user: { login: 'testuser' },
                body: 'Comment to delete',
            });

            let detail = null;
            el.addEventListener('comment-delete', (e) => {
                detail = e.detail;
            });

            el.querySelector('.delete-button').click();

            assertEqual(detail.id, 'comment-123');
        });
    });

    run("escapes HTML in user and body", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-1',
                user: { login: '<script>evil()</script>' },
                body: '<img onerror="xss">',
            });

            const author = el.querySelector('.comment-author');
            const body = el.querySelector('.comment-body');

            // Should be escaped, not contain actual script/img tags
            assertEqual(author.innerHTML.includes('<script>'), false);
            assertEqual(body.innerHTML.includes('<img'), false);
        });
    });

    run("renders nothing when comment is null", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment(null);

            assertEqual(el.innerHTML, '');
        });
    });

    run("Escape key in edit mode cancels edit", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-1',
                user: { login: 'testuser' },
                body: 'Original',
            });

            el.querySelector('.edit-button').click();

            const textarea = el.querySelector('textarea');
            const event = new KeyboardEvent('keydown', { key: 'Escape' });
            textarea.dispatchEvent(event);

            // Should be back in view mode
            assertEqual(el.querySelector('.comment-body') !== null, true);
        });
    });

    run("Ctrl+Enter in edit mode saves", ({ assertEqual }) => {
        withElement('review-comment', (el) => {
            el.setComment({
                id: 'comment-1',
                user: { login: 'testuser' },
                body: 'Original',
            });

            el.querySelector('.edit-button').click();

            let detail = null;
            el.addEventListener('comment-update', (e) => {
                detail = e.detail;
            });

            const textarea = el.querySelector('textarea');
            textarea.value = 'Updated via keyboard';
            const event = new KeyboardEvent('keydown', {
                key: 'Enter',
                ctrlKey: true,
            });
            textarea.dispatchEvent(event);

            assertEqual(detail.body, 'Updated via keyboard');
        });
    });
});
