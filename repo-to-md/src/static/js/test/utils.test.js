/**
 * Tests for utility functions in utils.js
 */

import {
    escapeHtml,
    escapeAttr,
    getFileName,
    formatDate,
    getRowType,
    groupCommentsByLine,
    getCommentsByFile,
    computeFileTreeItems,
} from '../utils.js';

// escapeHtml tests - Security critical
minitest("escapeHtml", ({ run, assertEqual }) => {
    run("escapes angle brackets", ({ assertEqual }) => {
        assertEqual(escapeHtml("<script>alert('xss')</script>"), "&lt;script&gt;alert('xss')&lt;/script&gt;");
    });

    run("escapes ampersands", ({ assertEqual }) => {
        assertEqual(escapeHtml("foo & bar"), "foo &amp; bar");
    });

    run("preserves quotes (only need escaping in attributes)", ({ assertEqual }) => {
        const result = escapeHtml('"quoted"');
        assertEqual(result, '"quoted"');
    });

    run("handles empty string", ({ assertEqual }) => {
        assertEqual(escapeHtml(""), "");
    });

    run("handles null", ({ assertEqual }) => {
        assertEqual(escapeHtml(null), "");
    });

    run("handles undefined", ({ assertEqual }) => {
        assertEqual(escapeHtml(undefined), "");
    });

    run("preserves normal text", ({ assertEqual }) => {
        assertEqual(escapeHtml("Hello World"), "Hello World");
    });
});

// escapeAttr tests - Security critical
minitest("escapeAttr", ({ run, assertEqual }) => {
    run("escapes double quotes", ({ assertEqual }) => {
        assertEqual(escapeAttr('test"value'), "test&quot;value");
    });

    run("escapes single quotes", ({ assertEqual }) => {
        assertEqual(escapeAttr("test'value"), "test&#39;value");
    });

    run("escapes both quote types", ({ assertEqual }) => {
        assertEqual(escapeAttr(`"it's"test`), "&quot;it&#39;s&quot;test");
    });

    run("handles empty string", ({ assertEqual }) => {
        assertEqual(escapeAttr(""), "");
    });

    run("handles null", ({ assertEqual }) => {
        assertEqual(escapeAttr(null), "");
    });

    run("preserves normal text", ({ assertEqual }) => {
        assertEqual(escapeAttr("normal-text"), "normal-text");
    });
});

// getRowType tests - Core diff rendering logic
minitest("getRowType", ({ run, assertEqual }) => {
    run("returns 'modified' for deletion+addition pair", ({ assertEqual }) => {
        const row = {
            old_line: { line_type: 'deletion' },
            new_line: { line_type: 'addition' },
        };
        assertEqual(getRowType(row), "modified");
    });

    run("returns 'context' for matching lines", ({ assertEqual }) => {
        const row = {
            old_line: { line_type: 'context' },
            new_line: { line_type: 'context' },
        };
        assertEqual(getRowType(row), "context");
    });

    run("returns 'deletion' for old_line only", ({ assertEqual }) => {
        const row = {
            old_line: { line_type: 'deletion' },
            new_line: null,
        };
        assertEqual(getRowType(row), "deletion");
    });

    run("returns 'addition' for new_line only", ({ assertEqual }) => {
        const row = {
            old_line: null,
            new_line: { line_type: 'addition' },
        };
        assertEqual(getRowType(row), "addition");
    });

    run("returns 'context' for both lines with non-modification types", ({ assertEqual }) => {
        const row = {
            old_line: { line_type: 'context' },
            new_line: { line_type: 'addition' },
        };
        assertEqual(getRowType(row), "context");
    });

    run("returns 'context' for empty row", ({ assertEqual }) => {
        const row = {
            old_line: null,
            new_line: null,
        };
        assertEqual(getRowType(row), "context");
    });
});

// groupCommentsByLine tests
minitest("groupCommentsByLine", ({ run, assertEqual }) => {
    run("groups comments by line number", ({ assertEqual }) => {
        const comments = [
            { line: 10, body: "comment 1" },
            { line: 10, body: "comment 2" },
            { line: 20, body: "comment 3" },
        ];
        const result = groupCommentsByLine(comments);
        assertEqual(result[10].length, 2);
        assertEqual(result[20].length, 1);
    });

    run("returns empty object for empty array", ({ assertEqual }) => {
        const result = groupCommentsByLine([]);
        assertEqual(result, {});
    });

    run("preserves comment order within line", ({ assertEqual }) => {
        const comments = [
            { line: 5, body: "first" },
            { line: 5, body: "second" },
        ];
        const result = groupCommentsByLine(comments);
        assertEqual(result[5][0].body, "first");
        assertEqual(result[5][1].body, "second");
    });
});

// getCommentsByFile tests
minitest("getCommentsByFile", ({ run, assertEqual }) => {
    run("groups comments by file path", ({ assertEqual }) => {
        const comments = [
            { path: "src/foo.js", line: 10, body: "comment 1" },
            { path: "src/foo.js", line: 20, body: "comment 2" },
            { path: "src/bar.js", line: 5, body: "comment 3" },
        ];
        const result = getCommentsByFile(comments);
        assertEqual(result["src/foo.js"].length, 2);
        assertEqual(result["src/bar.js"].length, 1);
    });

    run("puts global comments (line=null) under __general__", ({ assertEqual }) => {
        const comments = [
            { path: "src/foo.js", line: null, body: "global comment" },
            { path: "src/foo.js", line: 10, body: "file comment" },
        ];
        const result = getCommentsByFile(comments);
        assertEqual(result["__general__"].length, 1);
        assertEqual(result["src/foo.js"].length, 1);
    });

    run("always includes __general__ key even if empty", ({ assertEqual }) => {
        const comments = [
            { path: "src/foo.js", line: 10, body: "comment" },
        ];
        const result = getCommentsByFile(comments);
        assertEqual(result["__general__"].length, 0);
    });

    run("returns only __general__ for empty array", ({ assertEqual }) => {
        const result = getCommentsByFile([]);
        assertEqual(Object.keys(result).length, 1);
        assertEqual(result["__general__"].length, 0);
    });
});

// getFileName tests
minitest("getFileName", ({ run, assertEqual }) => {
    run("extracts filename from path", ({ assertEqual }) => {
        assertEqual(getFileName("src/components/Button.js"), "Button.js");
    });

    run("handles single filename (no path)", ({ assertEqual }) => {
        assertEqual(getFileName("README.md"), "README.md");
    });

    run("handles deep paths", ({ assertEqual }) => {
        assertEqual(getFileName("a/b/c/d/e/file.txt"), "file.txt");
    });

    run("handles trailing slash (returns original path)", ({ assertEqual }) => {
        // When pop() returns empty string, falls back to original path
        const result = getFileName("src/folder/");
        assertEqual(result, "src/folder/");
    });

    run("handles empty string", ({ assertEqual }) => {
        assertEqual(getFileName(""), "");
    });
});

// formatDate tests
minitest("formatDate", ({ run, assertEqual }) => {
    run("formats Unix timestamp correctly", ({ assertEqual }) => {
        // Use a known timestamp: Jan 1, 2024 00:00:00 UTC = 1704067200
        const result = formatDate(1704067200);
        // Result depends on locale, but should contain "2024"
        assertEqual(result.includes("2024"), true);
    });

    run("returns original value for invalid timestamp", ({ assertEqual }) => {
        assertEqual(formatDate("not-a-number"), "not-a-number");
    });

    run("handles string timestamps", ({ assertEqual }) => {
        const result = formatDate("1704067200");
        assertEqual(result.includes("2024"), true);
    });

    run("handles zero timestamp", ({ assertEqual }) => {
        const result = formatDate(0);
        // Should be Jan 1, 1970
        assertEqual(result.includes("1970"), true);
    });
});

// computeFileTreeItems tests
minitest("computeFileTreeItems", ({ run, assertEqual }) => {
    run("computes items with all properties", ({ assertEqual }) => {
        const files = [
            { path: "src/foo.js", status: "modified" },
            { path: "src/bar.js", status: "added" },
        ];
        const commentsByFile = {
            "src/foo.js": [{ id: "1" }, { id: "2" }],
            "__general__": [],
        };
        const viewedFiles = new Set(["src/foo.js"]);

        const result = computeFileTreeItems(files, commentsByFile, viewedFiles);

        assertEqual(result.length, 2);
        assertEqual(result[0].path, "src/foo.js");
        assertEqual(result[0].status, "modified");
        assertEqual(result[0].commentCount, 2);
        assertEqual(result[0].isViewed, true);
        assertEqual(result[1].path, "src/bar.js");
        assertEqual(result[1].status, "added");
        assertEqual(result[1].commentCount, 0);
        assertEqual(result[1].isViewed, false);
    });

    run("handles empty inputs", ({ assertEqual }) => {
        const result = computeFileTreeItems([], {}, new Set());
        assertEqual(result.length, 0);
    });

    run("handles missing file in commentsByFile", ({ assertEqual }) => {
        const files = [{ path: "src/new.js", status: "added" }];
        const commentsByFile = { "__general__": [] };
        const viewedFiles = new Set();

        const result = computeFileTreeItems(files, commentsByFile, viewedFiles);

        assertEqual(result[0].commentCount, 0);
        assertEqual(result[0].isViewed, false);
    });

    run("preserves file order", ({ assertEqual }) => {
        const files = [
            { path: "a.js", status: "modified" },
            { path: "z.js", status: "added" },
            { path: "m.js", status: "deleted" },
        ];
        const result = computeFileTreeItems(files, {}, new Set());

        assertEqual(result[0].path, "a.js");
        assertEqual(result[1].path, "z.js");
        assertEqual(result[2].path, "m.js");
    });
});
