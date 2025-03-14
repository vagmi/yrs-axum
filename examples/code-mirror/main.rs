use futures_util::StreamExt;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::sync::{Mutex, RwLock};
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, State}, response::IntoResponse, routing::get, serve::Listener, Router
};
use tower_http::services::ServeDir;
use yrs::sync::Awareness;
use yrs::{Doc, Text, Transact};
use yrs_axum::broadcast::BroadcastGroup;
use yrs_axum::ws::{AxumSink, AxumStream};
use yrs_axum::AwarenessRef;

const STATIC_FILES_DIR: &str = "examples/code-mirror/frontend/dist";

#[tokio::main]
async fn main() {
    // We're using a single static document shared among all the peers.
    let awareness: AwarenessRef = {
        let doc = Doc::new();
        {
            // pre-initialize code mirror document with some text
            let txt = doc.get_or_insert_text("codemirror");
            let mut txn = doc.transact_mut();
            txt.push(
                &mut txn,
                r#"function hello() {
  console.log('hello world');
}"#,
            );
        }
        Arc::new(RwLock::new(Awareness::new(doc)))
    };

    // open a broadcast group that listens to awareness and document updates
    // and has a pending message buffer of up to 32 updates
    let bcast = Arc::new(BroadcastGroup::new(awareness.clone(), 32).await);

    // Create a router with our WebSocket handler and static file service
    let app = Router::new()
        .route("/my-room", get(ws_handler))
        .fallback_service(ServeDir::new(STATIC_FILES_DIR))
        .with_state(bcast);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("Listening on {}", &listener.local_addr().unwrap());
    
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(bcast): State<Arc<BroadcastGroup>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| peer(socket, bcast))
}

async fn peer(ws: WebSocket, bcast: Arc<BroadcastGroup>) {
    let (sink, stream) = ws.split();
    let sink = Arc::new(Mutex::new(AxumSink::from(sink)));
    let stream = AxumStream::from(stream);
    let sub = bcast.subscribe(sink, stream);
    match sub.completed().await {
        Ok(_) => println!("broadcasting for channel finished successfully"),
        Err(e) => eprintln!("broadcasting for channel finished abruptly: {}", e),
    }
}
