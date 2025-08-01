#![deny(missing_docs)]

//! A bevy plugin for SpacetimeDB.

mod aliases;
mod channel_receiver;
mod events;
// mod old_plugin;
mod plugin;
mod stdb_connection;

pub use aliases::*;
pub use bevy_spacetimedb_macros::*;
pub use channel_receiver::AddEventChannelAppExtensions;
pub use events::*;
pub use plugin::*;
pub use stdb_connection::*;
