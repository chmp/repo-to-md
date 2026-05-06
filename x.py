# /// script
# requires-python = ">=3.14"
# dependencies = ["playwright==1.56.0", "texc"]
#
# [tool.uv.sources]
# texc = { path = "../texc", editable = true }
# ///
import asyncio
import contextlib
import os
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from threading import Thread
from typing import Annotated

from texc import main, sh, Help


async def default():
    await precommit()


async def precommit():
    """Run common tasks

    Runs: format, check, tests"""
    await format()
    await check()
    await test()


async def format():
    """Format the code"""
    await sh("cargo fmt")


async def check():
    """Perform code checks"""
    await sh(
        t"""cargo clippy
            --all-targets
            --all-features
            -- -D warnings
        """,
    )


async def doc(
    *,
    open: Annotated[bool, Help("open the docs after building")] = False,
):
    """Build the documentation"""
    await sh(
        t"""cargo doc
            --document-private-items
            --package repo-to-md
            {"--open" if open else None}
        """,
    )


async def test(
    *,
    backtrace: Annotated[bool, Help("enable Rust backtraces")] = False,
    full: Annotated[bool, Help("Test all targets and features")] = False,
):
    """Run tests"""
    await sh(
        t"""cargo test
            {"--all-targets" if full else None}
            {"--all-features" if full else None}
        """,
        RUST_BACKTRACE="1" if backtrace else None,
    )


async def frontend_test():
    """Run browser frontend tests"""
    from playwright.async_api import async_playwright

    static_dir = Path("repo-to-md/src/static").resolve()
    if not static_dir.exists():
        raise RuntimeError(f"static directory does not exist: {static_dir}")

    summaries: list[tuple[str, int, int, int]] = []
    console_messages: list[str] = []
    console_errors: list[str] = []
    page_errors: list[str] = []
    response_errors: list[str] = []
    diagnostics: dict[str, object] = {}

    class QuietHandler(SimpleHTTPRequestHandler):
        def log_message(self, format: str, *args: object) -> None:
            return

    handler = partial(QuietHandler, directory=static_dir)
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()

    url = f"http://127.0.0.1:{server.server_port}/test.html"

    try:
        async with async_playwright() as playwright:
            def on_console(message):
                text = message.text
                console_messages.append(f"{message.type}: {text}")
                if message.type == "error" and "Failed to load resource" not in text:
                    console_errors.append(f"{message.type}: {text}")

            launch_options = {}
            if executable_path := os.environ.get("PLAYWRIGHT_LAUNCH_OPTIONS_EXECUTABLE_PATH"):
                launch_options["executable_path"] = executable_path

            browser = await playwright.chromium.launch(**launch_options)
            page = await browser.new_page()

            page.on("pageerror", lambda error: page_errors.append(str(error)))
            page.on("console", on_console)
            page.on(
                "response",
                lambda response: response_errors.append(
                    f"{response.status} {response.url}"
                )
                if response.status >= 400 and "favicon.ico" not in response.url
                else None,
            )

            await page.goto(url, wait_until="networkidle")
            await page.wait_for_timeout(10000)
            diagnostics["minitest_type"] = await page.evaluate("typeof minitest")
            diagnostics["body_text"] = await page.evaluate("document.body.innerText")
            diagnostics["scripts"] = await page.evaluate(
                "[...document.scripts].map(script => ({ src: script.src, type: script.type }))"
            )
            diagnostics["resources"] = await page.evaluate(
                "[...performance.getEntriesByType('resource')].map(entry => entry.name)"
            )
            stored_summaries = await page.evaluate("window.frontendTestResults || []")
            for summary in stored_summaries:
                item = (
                    summary["name"],
                    int(summary["passed"]),
                    int(summary["errors"]),
                    int(summary["total"]),
                )
                if item not in summaries:
                    summaries.append(item)
            await browser.close()
    except Exception as error:
        if "Executable doesn't exist" in str(error):
            raise RuntimeError(
                "Playwright Chromium is not installed. Run "
                "`uv run python -m playwright install chromium` and retry."
            ) from error
        raise
    finally:
        server.shutdown()
        server.server_close()
        with contextlib.suppress(RuntimeError):
            await asyncio.to_thread(thread.join, 1)

    if page_errors:
        raise RuntimeError("frontend page errors:\n" + "\n".join(page_errors))
    if response_errors:
        raise RuntimeError("frontend response errors:\n" + "\n".join(response_errors))

    failed_summaries = [
        (name, passed, errors, total)
        for name, passed, errors, total in summaries
        if errors != 0
    ]
    if failed_summaries:
        details = "\n".join(
            f"{name}: passed={passed}, errors={errors}, total={total}"
            for name, passed, errors, total in failed_summaries
        )
        raise RuntimeError(f"frontend tests failed:\n{details}")

    if not summaries:
        extra = "\n".join(console_errors)
        if not extra:
            extra = "\n".join(console_messages)
        if diagnostics:
            extra = (extra + "\n" if extra else "") + f"diagnostics: {diagnostics}"
        raise RuntimeError(
            "frontend tests did not report any minitest summaries"
            + (f"\nconsole output:\n{extra}" if extra else "")
        )

    if console_errors:
        raise RuntimeError("frontend console errors:\n" + "\n".join(console_errors))

    total_passed = sum(passed for _, passed, _, _ in summaries)
    total_tests = sum(total for _, _, _, total in summaries)
    print(f"Frontend tests passed: {total_passed}/{total_tests}")


async def run(
    *args: Annotated[str, Help("arbitrary arguments forward to `mda-server`")],
):
    """Run an abitrary command of the server (use `--` to pass options)"""
    await sh(t"cargo run -p repo-to-md -- {args}", RUST_LOG="debug")


if __name__ == "__main__":
    main()
