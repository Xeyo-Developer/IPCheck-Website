use askama::Template;
use axum::{
    Router,
    extract::ConnectInfo,
    response::{Html, IntoResponse, Json},
    routing::{get, get_service},
};
use headers::HeaderMap;
use local_ip_address::local_ip;
use reqwest;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ip: String,
}

#[derive(Serialize, Deserialize)]
struct GeoLocation {
    country: Option<String>,
    region: Option<String>,
    city: Option<String>,
    org: Option<String>,
    timezone: Option<String>,
    postal: Option<String>,
}

#[derive(Serialize)]
struct IpInfo {
    ip: String,
    user_agent: String,
    headers: Vec<(String, String)>,
    geo: Option<GeoLocation>,
    connection_type: String,
    is_mobile: bool,
    is_tor: bool,
    is_vpn: bool,
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
            return ip.split(',').next().unwrap_or("").trim().to_string();
        }
    }

    addr.ip().to_string()
}

fn detect_connection_type(headers: &HeaderMap) -> String {
    if headers.get("CF-Connecting-IP").is_some() {
        "Cloudflare".to_string()
    } else if headers.get("X-Real-IP").is_some() {
        "Proxy/Load Balancer".to_string()
    } else if headers.get("X-Forwarded-For").is_some() {
        "Forwarded".to_string()
    } else {
        "Direct".to_string()
    }
}

fn is_mobile_device(user_agent: &str) -> bool {
    let mobile_patterns = [
        "Mobile",
        "Android",
        "iPhone",
        "iPad",
        "iPod",
        "BlackBerry",
        "Opera Mini",
    ];
    mobile_patterns
        .iter()
        .any(|pattern| user_agent.contains(pattern))
}

fn detect_tor(headers: &HeaderMap) -> bool {
    headers.get("X-Tor-Exit-Node").is_some() || headers.get("X-Tor").is_some()
}

fn detect_vpn(headers: &HeaderMap, ip: &str) -> bool {
    headers.get("X-VPN").is_some()
        || headers.get("X-Forwarded-For").is_some()
        || ip.starts_with("10.")
        || ip.starts_with("192.168.")
        || ip.starts_with("172.")
}

async fn get_geo_info(ip: &str) -> Option<GeoLocation> {
    if ip.starts_with("127.") || ip.starts_with("192.168.") || ip.starts_with("10.") {
        return None;
    }

    let client = reqwest::Client::new();
    let url = format!("http://ip-api.com/json/{}", ip);

    if let Ok(response) = client.get(&url).send().await {
        if let Ok(json) = response.json::<serde_json::Value>().await {
            return Some(GeoLocation {
                country: json["country"].as_str().map(|s| s.to_string()),
                region: json["regionName"].as_str().map(|s| s.to_string()),
                city: json["city"].as_str().map(|s| s.to_string()),
                org: json["org"].as_str().map(|s| s.to_string()),
                timezone: json["timezone"].as_str().map(|s| s.to_string()),
                postal: json["zip"].as_str().map(|s| s.to_string()),
            });
        }
    }
    None
}

async fn ip_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let client_ip = get_client_ip(&headers, addr);
    let template = IndexTemplate { ip: client_ip };
    Html(template.render().unwrap())
}

async fn api_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let client_ip = get_client_ip(&headers, addr);
    let user_agent = headers
        .get("User-Agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    let connection_type = detect_connection_type(&headers);
    let is_mobile = is_mobile_device(&user_agent);
    let is_tor = detect_tor(&headers);
    let is_vpn = detect_vpn(&headers, &client_ip);

    let mut relevant_headers = Vec::new();
    for (name, value) in headers.iter() {
        if let Ok(value_str) = value.to_str() {
            relevant_headers.push((name.to_string(), value_str.to_string()));
        }
    }

    let geo = get_geo_info(&client_ip).await;

    let ip_info = IpInfo {
        ip: client_ip,
        user_agent,
        headers: relevant_headers,
        geo,
        connection_type,
        is_mobile,
        is_tor,
        is_vpn,
    };

    Json(ip_info)
}

async fn plain_handler(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let client_ip = get_client_ip(&headers, addr);
    client_ip
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
        .route("/api", get(api_handler))
        .route("/plain", get(plain_handler))
        .nest_service("/static", get_service(ServeDir::new("static")))
        .fallback(get(ip_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    if let Ok(local_ip) = local_ip() {
        println!("🚀 Enhanced IP Check Server");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📍 Available endpoints:");
        println!("  🌐 Web UI:    http://localhost:{}", port);
        println!("  📊 API:       http://localhost:{}/api", port);
        println!("  📝 Plain IP:  http://localhost:{}/plain", port);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("  🌍 Network:   http://{}:{}", local_ip, port);
        println!("  🌍 Network:   http://{}:{}/api", local_ip, port);
        println!("  🌍 Network:   http://{}:{}/plain", local_ip, port);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    } else {
        println!("🚀 Enhanced IP Check Server running on port {}", port);
    }

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    ctrlc::set_handler(move || {
        println!("\n🛑 Shutting down server...");
        std::process::exit(0);
    })
    .expect("Error setting Ctrl-C handler");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
