# Amazing Adventures Toolkit

A small cross-platform terminal helper for the Steam title **Amazing Adventures Around
the World** (Steam AppID **3530**). It does two things, each fully reversible:

1. **Graphics improvement** — downloads the latest
   [cnc-ddraw](https://github.com/FunkyFr3sh/cnc-ddraw) release and installs it into the
   game folder. cnc-ddraw is a native DirectDraw reimplementation; it fixes the game
   feeling like ~30 fps and the flickering mouse cursor under Proton/Wine.
2. **Splash-screen skip** — zeroes three timing constants inside the game payload that is
   embedded in `AmazingAdventures2.exe`, so the PopCap → SpinTop → "Amazing Adventures"
   startup splashes flash past and the game goes straight to the menu.

Run it from inside the game folder and pick options from a menu, or use the
non-interactive subcommands.

## Usage

```text
amazing-adventures-toolkit                 # interactive menu
amazing-adventures-toolkit status
amazing-adventures-toolkit graphics apply
amazing-adventures-toolkit graphics remove
amazing-adventures-toolkit splash apply
amazing-adventures-toolkit splash remove

Options (also work with the menu):
  --dir <PATH>   game folder (default: the current directory)
  -y, --yes      assume "yes"; never prompt
      --offline  do not contact the network (affects `status` only)
      --no-color disable coloured output
```

Example — apply both improvements in the game folder:

```sh
cd "/path/to/steamapps/common/Amazing Adventures Around the World"
amazing-adventures-toolkit graphics apply
amazing-adventures-toolkit splash apply
```

## The Steam launch option (important)

Under Proton/Wine, cnc-ddraw is only actually used if Wine is told to prefer the local
DLL. In Steam: right-click the game → **Properties → Launch Options** and paste:

```text
WINEDLLOVERRIDES="ddraw=n,b" %command%
```

The toolkit prints this reminder whenever it installs cnc-ddraw. On native Windows this
is usually unnecessary — the local `ddraw.dll` is loaded automatically.

## What it touches

Everything happens in the game folder you run it from:

| Action | Files |
|---|---|
| `graphics apply` | `ddraw.dll`, `ddraw.ini`, `Shaders/…`, `cnc-ddraw config.exe` (extracted from the release zip). A pre-existing `ddraw.ini` is saved as `ddraw.ini.bak` first. |
| `graphics remove` | Deletes the above and restores `ddraw.ini.bak`, if present. |
| `splash apply` | Writes 6 bytes in `AmazingAdventures2.exe` and saves a pristine copy as `AmazingAdventures2.exe.aaatw-orig`. |
| `splash remove` | Restores the pristine `AmazingAdventures2.exe`. |
| any change | Updates a small `.aaatw-toolkit.json` marker recording versions/hashes. |

Notes:

- The splash patch is validated against the exact game build (SHA-256
  `275b8b27…c00e3` stock / `d920cde4…eb833` patched); an unrecognised executable is
  refused rather than modified.
- **Steam "Verify integrity of game files" will undo the splash patch** (the EXE is part
  of the Steam manifest). Just run `splash apply` again afterwards.
- cnc-ddraw could not be simpler to keep up to date: run `graphics apply` again to fetch
  the newest release.

## Building

Requires a recent stable Rust toolchain (edition 2024).

```sh
cargo build --release          # native binary
cargo test --all
```

The Windows binary cross-compiles from Linux with the GNU toolchain:

```sh
rustup target add x86_64-pc-windows-gnu
# e.g. on Arch: sudo pacman -S mingw-w64-gcc
cargo build --release --target x86_64-pc-windows-gnu
```

CI builds and tests Linux, Windows and macOS (see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml)). macOS is currently best-effort:
the code is platform-neutral but cnc-ddraw itself is a Windows DLL, so it only makes
sense there under Wine/CrossOver.

## How it works

The shipped `AmazingAdventures2.exe` is a PopCap DRM/Steam wrapper, not the game: it
extracts the real game (`popcapgame1.exe`) from a PE overlay on every launch and runs it
as a child process whose working directory is the game folder. That is why a local
`ddraw.dll` is picked up, and why the only durable place to patch the splash constants is
inside the wrapper's overlay.

With thanks to [cnc-ddraw](https://github.com/FunkyFr3sh/cnc-ddraw) by FunkyFr3sh.

## License

[MIT](LICENSE).
