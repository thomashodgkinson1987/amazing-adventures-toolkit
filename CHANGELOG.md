# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-21

### Added

- Interactive terminal menu (run with no arguments) and non-interactive
  subcommands (`status`, `graphics apply|remove`, `splash apply|remove`).
- Graphics improvement: download the latest
  [cnc-ddraw](https://github.com/FunkyFr3sh/cnc-ddraw) release from GitHub and
  extract it into the game folder; back up any pre-existing `ddraw.ini` before
  overwriting it.
- Splash-screen skip: apply/remove the three-constant patch inside the game
  payload embedded in `AmazingAdventures2.exe`, with SHA-256 verification of the
  stock and patched executables.
- Automatic pristine backup (`AmazingAdventures2.exe.aaatw-orig`) alongside the
  executable before patching.
- `.aaatw-toolkit.json` marker file recording installed versions so `status` can
  report state without re-downloading.
- Steam launch-option instructions for Proton/Wine (`WINEDLLOVERRIDES`).

[Unreleased]: https://github.com/thomashodgkinson1987/amazing-adventures-toolkit/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/thomashodgkinson1987/amazing-adventures-toolkit/releases/tag/v0.1.0
