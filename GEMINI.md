# Agent Instructions (Google Gemini CLI)

This file is automatically loaded by the Gemini CLI (`gemini` command) as project-level instructions.

## What to Do

- Check `CODEBASE.md` or survey the file tree before reading full file bodies.
- Make minimal, targeted changes scoped to the current task.
- Run `.\scripts\devkit.ps1 scan` to catch unhandled errors and style issues before committing.
- Run the project test suite after every code change.
- Use `git dft` (difftastic) to review structural changes before committing.
- Handle all error branches, null/None inputs, and boundary conditions.

## What Not to Do

- Do not write tests that only confirm code runs without crashing.
- Do not reference functions or libraries that do not exist in this repository.
- Do not silently ignore errors.
- Do not use emoji characters or em dashes in any output.
- Do not write inline comments explaining what code does.
- Do not push to git remote unless explicitly instructed.

## Consolidated Antigravity Skills

- `critic`: Unified zero-trust review rubric (invariants, security, performance, comments, style).
- `craft`: Test engineering (boundary matrices, property tests, mutation, fuzzing).
- `memory`: Shared codebase indexing, dependency tracing, context compression, and 3-tier docs.
- `git-flow`: Atomic commits, Conventional Commits, branch safety, and changelog sync.

## Commit Convention

Conventional Commits: `<type>(<scope>): <summary>`
Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`, `ci`, `style`, `build`
