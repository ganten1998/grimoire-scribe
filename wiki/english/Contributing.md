**English** | [日本語](../japanese/Contributing-ja.md)

# Contributing

Thank you for considering contributing to Grimoire! This is an open source project under the
GPL-3.0 license, and contributions of all kinds are welcome.

## Ways to contribute

### Reporting bugs

Issues are completely open — file a bug report any time, no permission needed.

- Search existing issues first to avoid duplicates
- Use clear, descriptive titles
- Include Kindle firmware version, jailbreak method, and steps to reproduce
- Attach `grimoire/grimoire.log` if available

### Feature requests

Feature requests in issues are completely open too — just open an issue and describe what you
want and why. No need to ask first; if it's a good fit for the project we'll discuss it there.

### Translating

Grimoire is available in English, Spanish and Japanese. See [Translating](Translating) to improve a
translation or add a language. All translations are under the project's GPL-3.0-or-later license.

### Code contributions

- You're open to create PRs — go ahead and send one whenever you have something working
- But if your PR has a conflict, **that's your problem, not ours**. Rebase and resolve it yourself
  before asking for a review; we won't fix merge conflicts for you
- The codebase is primarily in Rust (`rust/` crate) with some shell scripts and Python
- Run `make check` to verify your changes pass the automated checks
- See the [Makefile](https://github.com/ganten1998/grimoire-scribe/blob/main/Makefile) for
  available commands (`make test`, `make scenarios`, `make l10n-check`, etc.)
- Code is built with Rust 1.97.1, Zig 0.16.0, and `cargo-zigbuild 0.23.0` for the Kindle target

### Documentation

Docs are in the `docs/` directory and are written in Markdown. See the existing docs for style
conventions. Documentation updates are checked as part of the project's checks.

### LLM use

- Using LLMs to write code is fine — we don't ban it and we won't ask you to declare it
- But your output is yours: anything you put in a PR is **yours**, and it's your fault if it does
  something wrong
- "The model wrote it" is not an excuse. You're responsible for reviewing, understanding, and
  testing every line you submit
- Don't dump unreviewed generated code into a PR and expect us to catch its bugs for you

## Wiki for developers

**This project has a wiki section for Developers who want to contribute code.** The wiki contains
deeper technical guides, architecture documents, and advanced tutorials that don't fit in the main
contributing guide. Topics include: building from source, the Rust crate structure, Kindle target
compilation, the scenario runner, localization workflow, and CI/CD setup.

Start with [Building from Source](Building-from-Source) for the toolchains, make targets and tests.

## Development setup

1. **Fork the repository** and clone your fork
2. **Install prerequisites**:
   - Rust 1.97.1 with the Kindle target:
     `rustup toolchain install 1.97.1 --profile minimal --target armv7-unknown-linux-musleabihf`
   - Zig 0.16.0 and `cargo-zigbuild 0.23.0`
   - A C compiler (`clang`), make, curl, and unzip
   - Python 3.12+, Node.js 18+, and LuaJIT
3. **Build and test**:
   - `make check` runs the full check suite including unit tests, scenarios, localization checks,
     and more
   - `make test` runs just the Rust unit tests
   - `make scenarios` runs the scripted UI scenarios
4. **Make a change** and ensure `make check` passes
5. **Submit a pull request** with a clear description of what you changed and why

## Project structure

- `rust/` - The Rust source code crate
- `docs/` - Documentation (including how to build, test, and contribute)
- `tests/scenarios/` - Scripted UI test scenarios
- `po/` - Translation files
- `.github/workflows/` - CI/CD configuration

## Ground rules

- Keep it civil in issues and PRs
- Maintainers decide what gets merged; a PR being open doesn't mean it will be accepted
- Don't expect us to fix your branches, conflicts, or failing CI for you
