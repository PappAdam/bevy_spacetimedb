use crate::stdb::Reducer;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_spacetimedb::{
    ReducerResultEvent, StdbConnectedEvent, StdbConnection, StdbPlugin, register_reducers, tables,
};
use spacetimedb_sdk::{DbConnectionBuilder, ReducerEvent};

use crate::stdb::{DbConnection, LobbyTableAccess, UserTableAccess, create_lobby, set_name};

use crate::stdb::RemoteModule;

mod stdb;

#[derive(Clone, Debug)]
pub struct RegisterPlayerEvent {
    pub event: ReducerEvent<Reducer>,
    pub id: u64,
}

#[derive(Clone, Debug)]
pub struct GsRegisterEvent {
    pub event: ReducerEvent<Reducer>,
    pub ip: String,
    pub port: u16,
}

pub fn main() {
    App::new()
        .add_plugins((MinimalPlugins, LogPlugin::default()))
        .add_plugins(
            StdbPlugin::default()
                // These 2 functions have no declarations in the sdk, they need to be injected
                .with_build_fn(DbConnectionBuilder::build)
                .with_run_fn(DbConnection::run_threaded)
                // Settings like db name, uri, etc. can come here.
                
                // Did not work on this part yet, it remains unchanged.
                .with_events(
                    |plugin: &StdbPlugin<DbConnection, RemoteModule>, app, db, reducers| {
                        tables!(user, lobby);

                        register_reducers!(
                            on_create_lobby(ctx) => CreateLobbyEvent {
                                event: ctx.event.clone()
                            },
                            on_set_name(ctx, name) => SetNameEvent {
                                event: ctx.event.clone(),
                                name: name.clone()
                            }
                        );
                    },
                ),
        )
        .add_systems(Update, on_connected)
        .run();
}

fn on_connected(
    mut events: EventReader<StdbConnectedEvent>,
    stdb: Res<StdbConnection<DbConnection>>,
) {
    for _ in events.read() {
        info!("Connected to SpacetimeDB");

        stdb.subscribe()
            .on_applied(|_| info!("Subscription to lobby applied"))
            .on_error(|_, err| error!("Subscription to lobby failed for: {}", err))
            .subscribe("SELECT * FROM lobby");

        stdb.subscribe()
            .on_applied(|_| info!("Subscription to user applied"))
            .on_error(|_, err| error!("Subscription to user failed for: {}", err))
            .subscribe("SELECT * FROM user");
    }
}

#[derive(Clone, Debug)]
pub struct CreateLobbyEvent {
    pub event: ReducerEvent<Reducer>,
}

#[derive(Clone, Debug)]
pub struct SetNameEvent {
    pub event: ReducerEvent<Reducer>,
    pub name: String,
}
