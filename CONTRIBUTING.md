# Contributing

Thanks for taking a look! This is a small tool, but here is how the repository is
organised.

## Branching model

- **`main`** is the _live working copy_. It should always build, pass its tests and be
  in a releasable state. It is what users clone.
- **`develop`** is the _working space_ where integration happens.
- **Feature branches** are created **off `develop`** and are **merged back into
  `develop`** when complete (via a pull request). Name them descriptively, e.g.
  `feature/windows-cross-build`, `fix/status-offline`, `docs/launch-options`.
- `develop` is promoted to `main` when a release is cut.

```
main      ──●────────────────────●─────────────► (releases)
             ╲                  ╱
develop       ●──●────●────●────●              (integration)
                 ╲     ╱ ╲  ╱
feature/*         ●───●   ●●                   (short-lived branches)
```

## Versioning

This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(`MAJOR.MINOR.PATCH`). Every user-visible change gets a note in
[`CHANGELOG.md`](CHANGELOG.md) under `Unreleased`; at release time that section is
renamed to the new version and dated.

## Making a change

1. Branch off `develop`.
2. Keep commits focused and follow **Conventional Commits** (see
   [Commit messages](#commit-messages) below); reference issues where relevant.
3. Before pushing, run:

   ```sh
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test --all
   ```

   CI runs exactly these on Linux, Windows and macOS.
4. Open a pull request targeting **`develop`**.

## Commit messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
Write messages as:

```text
<type>(<optional scope>): <description>

[optional body]

[optional footer(s)]
```

Common types:

- `feat` — a new user-facing feature (a MINOR version bump)
- `fix` — a bug fix (a PATCH version bump)
- `docs`, `test`, `refactor`, `perf`, `style`, `build`, `ci`, `chore` — no version bump

Add a scope where it helps:

```text
feat(status): add an --offline flag
fix(cncddraw): restore ddraw.ini from the game folder
docs: explain the Steam launch option
```

Mark breaking changes with `!` after the type/scope (or a `BREAKING CHANGE:` footer) —
these map to a MAJOR version bump:

```text
feat(cli)!: rename the `graphics` subcommand
```

The types line up with [versioning](#versioning) and [`CHANGELOG.md`](CHANGELOG.md), so
release notes can be assembled from the commit history.

## Tests

Unit tests cover the pure logic (splash site classification/reconstruction, asset
selection, zip extraction) and run anywhere:

```sh
cargo test --all
```

There is also an end-to-end apply/remove round-trip test that needs a real game
executable. It is skipped unless you point it at a pristine copy:

```sh
AAATW_STOCK_EXE=/path/to/pristine/AmazingAdventures2.exe cargo test --all
```

## Safety rules

The tool modifies files in the game folder. When changing anything in this area:

- Never patch an executable that does not match a known SHA-256; refuse instead.
- Always take a pristine backup before modifying, and verify hashes before and after.
- Do not commit game binaries or large assets to this repository.
