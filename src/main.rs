use axum::{
    extract::Query,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use futures::Stream;
use serde::Deserialize;
use std::{convert::Infallible, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Deserialize)]
struct WalletParams {
    address: String,
}

#[tokio::main]
async fn main() {
    // Создаем роутер с SSE endpoint
    let app = Router::new()
        .route("/sse", get(sse_handler))
        // Добавляем CORS для работы с фронтендом
        .layer(tower_http::cors::CorsLayer::permissive());

    // Запускаем сервер на порту 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000")
        .await
        .expect("Failed to bind to address");
    
    println!("🚀 Server started on http://0.0.0.0:4000");
    println!("📡 SSE endpoint: http://localhost:4000/sse?address=YOUR_WALLET_ADDRESS");
    
    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

async fn sse_handler(Query(params): Query<WalletParams>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let address = params.address.clone();
    
    println!("📥 Received SSE connection for address: {}", address);
    let (tx, rx) = mpsc::unbounded_channel();
    
    // Сразу отправляем событие "success"
    let success_event = Event::default()
        .data(format!(r#"{{"status": "success", "address": "{}"}}"#, address));
    
    tx.send(Ok(success_event))
        .expect("Failed to send initial event");
    
    // Преобразуем receiver в stream
    let stream = UnboundedReceiverStream::new(rx);
    
    // Настраиваем keep-alive для поддержания соединения
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive-text"),
    )
}
