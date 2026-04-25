//! `beast-serde` — save/load, replay journaling, and deterministic
//! serialization for the simulation.
//!
//! # Sprint scope
//!
//! Sprint S7 builds this crate up incrementally:
//!
//! * **S7.1** — [`SaveFile`] struct + bincode/serde_json round-trip (this story).
//! * **S7.2** — `SaveManager` to capture from / hydrate into [`beast_sim::Simulation`].
//! * **S7.3** — `ReplayJournal` for input sequence logging.
//! * **S7.4** — `SaveValidator` rejecting forbidden UI-derived keys.
//! * **S7.5** — `MigrationRegistry` for schema-version upgrades on load.
//! * **S7.6** — Cross-process determinism replay test (M2 Determinism milestone).
//!
//! # Determinism contract
//!
//! Every byte written by this crate must be a pure function of the
//! sim state at the captured tick. Concretely:
//!
//! * `BTreeMap`/`BTreeSet` everywhere a key-set is iterated — never
//!   `HashMap`/`HashSet`. INVARIANTS §1 forbids order-leaking iteration
//!   into hashed state, and the save file is a hash input for replay
//!   gates.
//! * Integer fields use Q32.32 fixed-point (via `beast_core::Q3232`) for
//!   continuous quantities; floats are forbidden on the sim path and
//!   `clippy::float_arithmetic = "deny"` enforces this for the crate.
//! * `bincode` 2.x with `config::standard()` produces a byte-stable
//!   wire format suitable for the determinism gate (see `save` module
//!   docs).

#![forbid(unsafe_code)]

pub mod manager;
pub mod replay;
pub mod save;

pub use manager::{
    load_from_path, load_game, primitive_fingerprint, save_game, save_to_path, ManagerError,
};
pub use replay::{InputEvent, ReplayError, ReplayJournal, REPLAY_FORMAT_VERSION};
pub use save::SaveFile;
