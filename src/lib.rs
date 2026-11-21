//! Amazing Adventures Toolkit.
//!
//! A small cross-platform terminal helper for the Steam title *Amazing Adventures
//! Around the World* (AppID 3530). It does two things, each reversible:
//!
//! 1. **Graphics improvement** — downloads the latest [cnc-ddraw] release and extracts
//!    it into the game folder (native DirectDraw replacement; fixes the ~30 fps feel and
//!    the flickering cursor under Proton/Wine).
//! 2. **Splash-screen skip** — zeroes three timing constants inside the game payload that
//!    is embedded in `AmazingAdventures2.exe`, so the startup splashes flash past.
//!
//! The binary is orchestrated from [`main.rs`]; this library exposes the modules so they
//! can also be unit/integration tested.
//!
//! [cnc-ddraw]: https://github.com/FunkyFr3sh/cnc-ddraw

#![forbid(unsafe_code)]

pub mod app;
pub mod cli;
pub mod cncddraw;
pub mod game_dir;
pub mod hashes;
pub mod menu;
pub mod splash;
pub mod state;
