# Agent Instructions & Verification Standards

Generic drop-in instruction file for AI coding agents (Cursor, Aider, Copilot, etc.).
For Claude Code, use `CLAUDE.md`. For Gemini CLI, use `GEMINI.md`.

## What to Do

- Read `CODEBASE.md` before reading full file bodies.
- Make minimal, targeted changes scoped to the current task.
- Run `.\scripts\devkit.ps1 scan` to statically verify diff changes.
- Run the project test suite after every code change.
- Stage only files relevant to the current change before committing.
- Always handle error branches, null/None inputs, and boundary conditions.

## What Not to Do

- Do not write tests that only confirm code runs without crashing.
- Do not reference libraries or functions that do not exist in this repository.
- Do not silently swallow errors with empty `catch` blocks or unvalidated defaults.
- Do not use emoji characters or em dashes in any output.
- Do not write inline comments. Do not write comment blocks explaining what code does line-by-line.
- Do not push to git remote unless explicitly instructed.

## Commit Convention

Conventional Commits format: `<type>(<scope>): <summary>`
Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`, `ci`, `style`, `build`
No emojis. No em dashes. No trailing periods. Imperative mood.

## Build & Test Commands

[FILL IN - e.g., `cargo test`, `npm test`, `pytest`, `go test ./...`]
