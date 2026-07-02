# Contributing to Orlando

Thank you for your interest in contributing to Orlando!

## Contributor License Agreement (CLA)

All contributors must sign the
[Industrial Algebra Contributor License Agreement (CLA)](https://github.com/Industrial-Algebra/.github/blob/main/CLA.md)
before their contributions can be merged.

The CLA grants Industrial Algebra the right to relicense your contributions
(non-exclusive, worldwide, royalty-free, irrevocable) and includes a patent
grant. Once signed, it covers all Industrial Algebra projects — no per-project
signing is required.

## Licensing

Orlando is licensed under **Apache-2.0**. See [LICENSE](LICENSE) for the full
text. Every source file carries the header:

```
// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0
```

## Development workflow

1. Fork the repository and create a feature branch from `develop`.
2. Ensure `cargo test` and `cargo clippy` pass.
3. Add tests for new functionality.
4. Open a pull request against `develop`.

For WASM work, verify `cargo check --target wasm32-unknown-unknown` passes.

## Branching & release flow

Orlando follows the Industrial Algebra gitflow. **`develop` is the single
source of truth**; `main` receives releases only.

```
feature branch  --PR-->  develop  --release PR-->  main  --tag v*.*.*--
```

- **Feature work** branches off `develop` and PRs back to `develop`.
- **Releases** are a PR from `develop` (or a `release/*` branch off `develop`)
  into `main`, then a `v*.*.*` tag on `main` (which triggers publishing).
- **`main` must never receive commits that are not also on `develop`.** This is
  the rule that keeps the two branches from diverging. Release-only changes
  (CI, packaging, version bumps) are made on `develop` first, then flow to
  `main` via the release PR — never the reverse without an explicit back-merge.
- If a hotfix is needed on `main`, open it against `develop` and release it
  forward, or back-merge `main` → `develop` immediately afterward.

> **Why this matters:** prior to 0.6.0, release infra landed on `main`
> without flowing back to `develop`, causing the branches to diverge for two
> major versions and making later releases require messy conflict resolution.
> The rule above prevents that recurrence.
