# /// script
# requires-python = ">=3.14"
# dependencies = ["texc"]
#
# [tool.uv.sources]
# texc = { path = "../texc", editable = true }
# ///
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


async def run(
    *args: Annotated[str, Help("arbitrary arguments forward to `mda-server`")],
):
    """Run an abitrary command of the server (use `--` to pass options)"""
    await sh(t"cargo run -p repo-to-md -- {args}", RUST_LOG="debug")


if __name__ == "__main__":
    main()
