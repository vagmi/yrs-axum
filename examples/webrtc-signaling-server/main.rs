use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use tower_http::services::ServeDir;
use yrs_axum::signaling::{signaling_conn, SignalingService};

const STATIC_FILES_DIR: &str = "examples/webrtc-signaling-server/frontend/dist";

#[tokio::main]
async fn main() {
    let signaling = SignalingService::new();

    let app = Router::new()
        .route("/signaling", get(ws_handler))
        .fallback_service(ServeDir::new(STATIC_FILES_DIR))
        .with_state(signaling);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(svc): State<SignalingService>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| peer(socket, svc))
}

async fn peer(ws: WebSocket, svc: SignalingService) {
    println!("new incoming signaling connection");
    match signaling_conn(ws, svc).await {
        Ok(_) => println!("signaling connection stopped"),
        Err(e) => eprintln!("signaling connection failed: {}", e),
    }
}
