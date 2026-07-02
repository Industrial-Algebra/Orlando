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
