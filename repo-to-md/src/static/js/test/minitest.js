// (c) 2023 - 2024 Christopher Prohm
/**
 * Minimal JS testing
 *
 *
 * Minitest executes the supplied test. Test execution logs to the javascript
 * console. Minitest supports both synchronous and asynchronous tests (with
 * timeouts).
 *
 * Usage:
 *
 * ```javascript
 * minitest("single test", () => {
 *    // ...
 * });
 *
 * minitest("test with examples", [example1, example2], (example) => {
 *    // ...
 * })
 * ```
 *
 * Asynchronous tests are supported as well. Per default a timeout on 1 second
 * is used. To configure the timeout supply examples (or a single example) with
 * a timeout key. Note, that asynchronous execution cannot be cancelled in
 * javascript. Therefore, the test code will continue executing even after the
 * timeout.
 *
 * ```javascript
 * minitest("async test", async () => {
 *    // ...
 * })
 *
 * minitest("async test with timeout", { timeout: 2000 }, async () => {
 *    // ...
 * })
 * ```
 *
 * Using provided assertions:
 *
 * ```javascript
 * minitest("assert equality", ({assertEqual}) => {
 *    assertEqual(true, 1 == 2);
 * });
 *
 * minitest("errors can be asserted", ({assertThrows}) => {
 *    assertThrows(() => {
 *       throw new Error();
 *    });
 * })
 * ```
 *
 * While minitest can be called recursively, it will most likely behave not as
 * intended. To support asynchronous tests, minitest executes all test in a
 * promise one after the other. Therefore nested minitest calls will execute
 * after the test function has ended.
 */
const minitest = (() => {
    "use strict";

    globalThis.__minitest_results = [];

    // adapted from https://stackoverflow.com/a/53593328
    const stableStringify = obj => {
        const keys = {};
        JSON.stringify(obj, (key, value) => {
            keys[key] = null;
            return value;
        });
        return JSON.stringify(obj, Object.keys(keys).sort());
    };

    const failAfter = timeout => new Promise((_, reject) => setTimeout(reject, timeout, new Error("timeout exceeded")));

    const newContext = (parent) => {
        const ctx = {
            chain: Promise.resolve(null),
            children: [],
            error: null,
        };
        if (parent) {
            parent.children.push(ctx);
        }
        return ctx;
    };

    const summarizeResults = ctx => {
        const res = {
            passed: +(ctx.error == null),
            errors: +(ctx.error != null),
        };

        for (const child of ctx.children) {
            const childSummary = summarizeResults(child);
            res.passed += childSummary.passed;
            res.errors += childSummary.errors;
        }

        return { total: res.passed + res.errors, ...res };
    }

    const scope = ctx => ({
        assertEqual: (a, b) => {
            a = stableStringify(a);
            b = stableStringify(b);
            if (a != b) {
                throw new Error(`${a} != ${b}\n\nleft:  ${a}\nright: ${b}`);
            }
        },
        assertThrows: (func, ...params) => {
            try {
                func(...params)
            } catch (e) {
                return;
            }
            throw new Error(`Expected function ${func} to throw and error`);
        },
        run: function run(optsOrLabel, testFunc) {
            ctx.chain = ctx.chain.finally(async () => {
                const opts = (optsOrLabel.constructor === Object) ? { ...optsOrLabel } : { name: optsOrLabel };
                const console = (minitest && minitest.console) || window.console;
                const testName = opts.name || stableStringify(opts);
                const localCtx = newContext(ctx, testName);

                console.groupCollapsed(`[${testName}]`);
                try {
                    await Promise.race([
                        testFunc({ ...scope(localCtx), ...opts }),
                        failAfter(opts.timeout || 1000),
                    ]);
                } catch (err) {
                    localCtx.error = err;
                }

                await localCtx.chain;

                if (localCtx.error) {
                    console.error(`[${testName}]`, localCtx.error);
                }
                console.groupEnd();

                const summary = summarizeResults(localCtx);
                const style = (summary.errors == 0) ? "color: #070;" : "color: #a00;";
                console.log(
                    `%c[%s] passed: %d, errors: %d, total: %d`,
                    style, testName, summary.passed, summary.errors, summary.total,
                );
                globalThis.__minitest_results.push({
                    name: testName,
                    passed: summary.passed,
                    errors: summary.errors,
                    total: summary.total,
                });
            });
        },
    });

    return scope(newContext(null)).run;
})();

globalThis.minitest = minitest;
