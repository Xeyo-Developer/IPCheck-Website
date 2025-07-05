use axum::{
    extract::ConnectInfo,
    response::{Html, IntoResponse},
    routing::{get, get_service},
    Router,
};
use askama::Template;
use headers::HeaderMap;
use std::net::SocketAddr;
use tower_http::{services::ServeDir, trace::TraceLayer};
use local_ip_address::local_ip;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ip: String,
}

fn get_client_ip(headers: &HeaderMap, addr: SocketAddr) -> String {
    if let Some(cf_ip) = headers.get("CF-Connecting-IP") {
        if let Ok(ip) = cf_ip.to_str() {
            return ip.to_string();
        }
    }

    if let Some(real_ip) = headers.get("X-Real-IP") {
        if let Ok(ip) = real_ip.to_str() {
            return ip.to_string();
        }
    }

    if let Some(forwarded_for) = headers.get("X-Forwarded-For") {
        if let Ok(ip) = forwarded_for.to_str() {
            return ip.split(',')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
        }
    }

    addr.ip().to_string()
}

async fn ip_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let client_ip = get_client_ip(&headers, addr);
    let template = IndexTemplate { ip: client_ip };
    Html(template.render().unwrap())
}


#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port = if args.len() > 1 {
        args[1].parse::<u16>().unwrap_or(3000)
    } else {
        3000
    };

    let app = Router::new()
        .route("/", get(ip_handler))
        .nest_service("/static", get_service(ServeDir::new("static")))
        .fallback(get(ip_handler))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    if let Ok(local_ip) = local_ip() {
        println!("Serwer dostępny na:");
        println!("  http://localhost:{}", port);
        println!("  http://{}:{}", local_ip, port);
    } else {
        println!("Serwer działa na porcie {}", port);
    }

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    ctrlc::set_handler(move || {
        println!("\nZamykanie serwera...");
        std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}