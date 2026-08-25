# AI Agent Development & Verification Guidelines

## Core Principles

1. **Zero-Trust Verification**: Never declare a task complete solely because code compiles or runs without immediate errors.
2. **Real Assertions Only**: Unit tests must assert concrete output values, not just exit code 0 or trivial loops.
3. **Handle Failure Paths**: Every error branch, negative input, and boundary condition must be explicitly handled.
4. **Scope Before Reading**: Read `CODEBASE.md` or survey the file tree before reading full file bodies.
5. **Minimal Comments**: No inline comments. No comment blocks that restate what code does. Docstrings for public Python functions and classes only.
6. **No Emojis or Em Dashes**: Never use emoji characters or em dashes in code, comments, commit messages, or responses.

## Core Slash Commands (Claude Code)

| Command | Lifecycle | Purpose |
| :--- | :--- | :--- |
| `/init` | Setup | Initializes repository, constructs `CODEBASE.md`, and installs hooks. |
| `/plan` | Start Task | Scopes candidate files, traces callers, and forecasts token budget. |
| `/ship` | Finish Task | Full release pipeline: scan -> sanitize -> test -> commit -> changelog & map sync. |
| `/review` | Inspection | Adversarial zero-trust code audit (invariants + security + performance + style). |
| `/verify` | Verification | Fast in-diff mutation test to prove test validity. |
| `/compact` | Memory | Mid-session context compression & handoff logging to `DECISIONS.md`. |
| `/doc` | Documentation | Generate slim 3-tier docs (`/doc readme`, `/doc arch`, `/doc dev`) with Mermaid. |

## Consolidated Antigravity Skills

- `skills/critic`: Unified zero-trust review rubric (invariants, security, performance, comments, style).
- `skills/craft`: Test engineering (boundary matrices, property tests, mutation, fuzzing).
- `skills/memory`: Shared codebase indexing, dependency tracing, context compression, and 3-tier docs.
- `skills/git-flow`: Atomic commits, Conventional Commits, branch safety, and changelog sync.
