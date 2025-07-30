use std::{
    any::{Any, TypeId},
    sync::{
        Mutex,
        mpsc::{Sender, channel},
    },
};

use bevy::{
    app::{App, Plugin},
    platform::collections::HashMap,
};
use spacetimedb_sdk::{DbConnectionBuilder, DbContext, Table, TableWithPrimaryKey};

use crate::{
    DeleteEvent, InsertEvent, InsertUpdateEvent, ReducerResultEvent, StdbConnectedEvent,
    StdbConnection, StdbConnectionErrorEvent, StdbDisconnectedEvent, UpdateEvent,
    channel_receiver::AddEventChannelAppExtensions,
};

/// A function that registers callbacks for events.
pub type FnRegisterCallbacks<T, M> =
    fn(&StdbPlugin<T, M>, &mut App, &<T as DbContext>::DbView, &<T as DbContext>::Reducers);

/// A plugin for SpacetimeDB connections.
pub struct StdbPlugin<T: DbContext, M: spacetimedb_sdk::__codegen::SpacetimeModule> {
    /// A function that builds a connection to the database.
    register_events: Option<FnRegisterCallbacks<T, M>>,
    event_senders: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
    build_fn: Option<fn(DbConnectionBuilder<M>) -> spacetimedb_sdk::Result<T>>,
    run_fn: Option<fn(&T) -> std::thread::JoinHandle<()>>,
}

impl<T: DbContext, M: spacetimedb_sdk::__codegen::SpacetimeModule> StdbPlugin<T, M> {
    /// Adds a function to register all events required by a Bevy application
    pub fn with_events(mut self, register_callbacks: FnRegisterCallbacks<T, M>) -> Self {
        self.register_events = Some(register_callbacks);
        self
    }

    /// Adds a builder function to build the StdbConnection with.
    pub fn with_build_fn(
        mut self,
        build_fn: fn(DbConnectionBuilder<M>) -> spacetimedb_sdk::Result<T>,
    ) -> Self {
        self.build_fn = Some(build_fn);
        self
    }

    /// TODO!
    pub fn with_run_fn(mut self, run_fn: fn(&T) -> std::thread::JoinHandle<()>) -> Self {
        self.run_fn = Some(run_fn);
        self
    }

    /// Register a Bevy event of type InsertEvent<TRow> for the `on_insert` event on the provided table.
    pub fn on_insert<TRow>(&self, app: &mut App, table: impl Table<Row = TRow>) -> &Self
    where
        TRow: Send + Sync + Clone + 'static,
    {
        let type_id = TypeId::of::<InsertEvent<TRow>>();

        let mut map = self.event_senders.lock().unwrap();

        let sender = map
            .entry(type_id)
            .or_insert_with(|| {
                let (send, recv) = channel::<InsertEvent<TRow>>();
                app.add_event_channel(recv);
                Box::new(send)
            })
            .downcast_ref::<Sender<InsertEvent<TRow>>>()
            .expect("Sender type mismatch")
            .clone();

        table.on_insert(move |_ctx, row| {
            let event = InsertEvent { row: row.clone() };
            let _ = sender.send(event);
        });

        self
    }

    /// Register a Bevy event of type DeleteEvent<TRow> for the `on_delete` event on the provided table.
    pub fn on_delete<TRow>(&self, app: &mut App, table: impl Table<Row = TRow>) -> &Self
    where
        TRow: Send + Sync + Clone + 'static,
    {
        let type_id = TypeId::of::<DeleteEvent<TRow>>();

        let mut map = self.event_senders.lock().unwrap();
        let sender = map
            .entry(type_id)
            .or_insert_with(|| {
                let (send, recv) = channel::<DeleteEvent<TRow>>();
                app.add_event_channel(recv);
                Box::new(send)
            })
            .downcast_ref::<Sender<DeleteEvent<TRow>>>()
            .expect("Sender type mismatch")
            .clone();

        table.on_delete(move |_ctx, row| {
            let event = DeleteEvent { row: row.clone() };
            let _ = sender.send(event);
        });

        self
    }

    /// Register a Bevy event of type UpdateEvent<TRow> for the `on_update` event on the provided table.
    pub fn on_update<TRow, TTable>(&self, app: &mut App, table: TTable) -> &Self
    where
        TRow: Send + Sync + Clone + 'static,
        TTable: Table<Row = TRow> + TableWithPrimaryKey<Row = TRow>,
    {
        let type_id = TypeId::of::<UpdateEvent<TRow>>();

        let mut map = self.event_senders.lock().unwrap();
        let sender = map
            .entry(type_id)
            .or_insert_with(|| {
                let (send, recv) = channel::<UpdateEvent<TRow>>();
                app.add_event_channel(recv);
                Box::new(send)
            })
            .downcast_ref::<Sender<UpdateEvent<TRow>>>()
            .expect("Sender type mismatch")
            .clone();

        table.on_update(move |_ctx, old, new| {
            let event = UpdateEvent {
                old: old.clone(),
                new: new.clone(),
            };
            let _ = sender.send(event);
        });

        self
    }

    /// Register a Bevy event of type InsertUpdateEvent<TRow> for the `on_insert` and `on_update` events on the provided table.
    pub fn on_insert_update<TRow, TTable>(&self, app: &mut App, table: TTable) -> &Self
    where
        TRow: Send + Sync + Clone + 'static,
        TTable: Table<Row = TRow> + TableWithPrimaryKey<Row = TRow>,
    {
        let type_id = TypeId::of::<InsertUpdateEvent<TRow>>();

        let mut map = self.event_senders.lock().unwrap();
        let send = map
            .entry(type_id)
            .or_insert_with(|| {
                let (send, recv) = channel::<InsertUpdateEvent<TRow>>();
                app.add_event_channel(recv);
                Box::new(send)
            })
            .downcast_ref::<Sender<InsertUpdateEvent<TRow>>>()
            .expect("Sender type mismatch")
            .clone();

        let send_update = send.clone();
        table.on_update(move |_ctx, old, new| {
            let event = InsertUpdateEvent {
                old: Some(old.clone()),
                new: new.clone(),
            };
            let _ = send_update.send(event);
        });

        table.on_insert(move |_ctx, row| {
            let event = InsertUpdateEvent {
                old: None,
                new: row.clone(),
            };
            let _ = send.send(event);
        });

        self
    }

    /// Register a Bevy event of type ReducerResultEvent<TReducer> for the `on_<reducer_name>` event on the provided reducers.
    pub fn reducer_event<TReducer>(&self, app: &mut App) -> Sender<ReducerResultEvent<TReducer>>
    where
        TReducer: Send + Sync + Clone + 'static,
    {
        let (send, recv) = channel::<ReducerResultEvent<TReducer>>();
        app.add_event_channel(recv);

        send
    }
}

impl<T: DbContext + Send + Sync + 'static, M: spacetimedb_sdk::__codegen::SpacetimeModule> Plugin
    for StdbPlugin<T, M>
{
    fn build(&self, app: &mut App) {
        let (send_connected, recv_connected) = channel::<StdbConnectedEvent>();
        let (send_disconnected, recv_disconnected) = channel::<StdbDisconnectedEvent>();
        let (send_connect_error, recv_connect_error) = channel::<StdbConnectionErrorEvent>();

        app.add_event_channel::<StdbConnectionErrorEvent>(recv_connect_error)
            .add_event_channel::<StdbConnectedEvent>(recv_connected)
            .add_event_channel::<StdbDisconnectedEvent>(recv_disconnected);

        // let conn_builder = self
        //     .connection_builder
        //     .as_ref()
        //     .expect("Connection builder is not set, use with_connection() method");
        // let conn = conn_builder(send_connected, send_disconnected, send_connect_error, app);

        let conn_builder = DbConnectionBuilder::new();

        let conn_builder = conn_builder
            .with_module_name("tic")
            .with_uri("http://localhost:3000")
            .on_connect_error(move |_ctx, err| {
                send_connect_error
                    .send(StdbConnectionErrorEvent { err })
                    .unwrap();
            })
            .on_disconnect(move |_ctx, err| {
                send_disconnected
                    .send(StdbDisconnectedEvent { err })
                    .unwrap();
            })
            .on_connect(move |_ctx, id, token| {
                send_connected
                    .send(StdbConnectedEvent {
                        identity: id,
                        access_token: token.to_string(),
                    })
                    .unwrap();
            });

        let build_fn = self
            .build_fn
            .ok_or("Build function must be set in StdbPlugin")
            .unwrap();

        let conn = build_fn(conn_builder).unwrap();

        let run_fn = self
            .run_fn
            .ok_or("Run function must be set in StdbPlugin")
            .unwrap();

        if let Some(register_callbacks) = self.register_events {
            register_callbacks(self, app, conn.db(), conn.reducers());
        }

        run_fn(&conn);

        app.insert_resource(StdbConnection::new(conn));
    }
}
