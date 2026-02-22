use axum::extract::OriginalUri;
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use http::Method;
use std::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{sleep, Duration};

async fn ok() -> impl IntoResponse {
    "ok"
}

async fn err() -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "fail")
}

// /sleep/150
async fn sleep_ms(axum::extract::Path(ms): axum::extract::Path<u64>) -> impl IntoResponse {
    sleep(Duration::from_millis(ms)).await;
    "slept"
}

async fn redir() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(header::LOCATION, "/ok".parse().unwrap());
    (StatusCode::FOUND, headers, "")
}

/// Возвращает (base_url, shutdown_sender, join_handle)
pub fn spawn_test_server() -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
    // 1) Listener на случайном свободном порту
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    async fn fallback(method: Method, uri: OriginalUri) -> String {
        format!("fallback: {} {}", method, uri.0)
    }

    // 2) Роуты
    let app = Router::new()
        .route("/ok", get(ok).put(ok).delete(ok))
        .route("/err", get(err).put(err).delete(err))
        .route("/sleep/{ms}", get(sleep_ms).patch(sleep_ms))
        .route("/redir", get(redir))
        .fallback(fallback);

    // 3) Канал для graceful shutdown
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // 4) Запуск в фоне
    let handle = tokio::spawn(async move {
        let server = axum::serve(
            tokio::net::TcpListener::from_std(listener).unwrap(),
            app,
        )
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            });

        // Если сервер упадёт — тест должен это увидеть
        server.await.unwrap();
    });

    (base_url, shutdown_tx, handle)
}

pub async fn wait_until_ready(url: &str) {
    let client = reqwest::Client::new();
    for _ in 0..20 {
        if client.get(format!("{}/ok", url)).send().await.is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("test server not ready");
}
