# External & Local Dependency Graph (DEPS.md)

> Maps runtime toolchain environments and cross-repository local package links for AI agents.

---

## 1. Runtime Toolchain & Environment
<!-- Specify any non-global interpreters, conda environments, or virtualenvs -->
- **Primary Runtime**: [e.g. Node 22 / Python 3.12 / Rust 1.80 / Go 1.22]
- **Environment Manager**: [e.g. Conda env: `my-conda-env` | Virtualenv: `.venv/` | nvm: `22`]
- **Activation Command**: `[e.g. conda activate my-conda-env / source .venv/bin/activate]`

---

## 2. Local Cross-Repo Package Links
<!-- When this project depends on another local repo, list it here so agents read its CODEBASE.md -->

| Package Name | Local Relative Path | Code Map Path | Purpose |
| :--- | :--- | :--- | :--- |
| `[e.g. my-core-lib]` | `../my-core-lib` | `../my-core-lib/CODEBASE.md` | Core domain types and utilities |
| `[e.g. shared-ui]`   | `../shared-ui`   | `../shared-ui/CODEBASE.md`   | Shared frontend component library |

---

## 3. Agent Instructions for Linked Packages
When importing a symbol from any package listed above:
1. Do **not** recursively crawl the linked package's source directory.
2. Read the linked package's `CODEBASE.md` (<300 tokens) to find exact function signatures and exported types.
3. Jump directly to specific line ranges only if deep implementation logic is needed.
