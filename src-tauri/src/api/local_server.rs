use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use tauri::AppHandle;

use crate::integrations::codex::{self, AgentEvent};

const MAX_REQUEST_BYTES: usize = 64 * 1024;
const EVENT_PATH: &str = "/api/v1/events";
const HEALTH_PATH: &str = "/api/v1/health";

pub fn start(app: AppHandle, expected_token: String) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", codex::EVENT_PORT)).map_err(|error| {
        log_server_error("bind_failed", &error.to_string());
        format!("failed to bind AgentLight loopback API: {error}")
    })?;

    thread::Builder::new()
        .name("agentlight-local-api".to_string())
        .spawn(move || {
            for incoming in listener.incoming() {
                let stream = match incoming {
                    Ok(stream) => stream,
                    Err(error) => {
                        log_server_error("accept_failed", &error.to_string());
                        continue;
                    }
                };
                let app = app.clone();
                let expected_token = expected_token.clone();

                if let Err(error) = thread::Builder::new()
                    .name("agentlight-local-request".to_string())
                    .spawn(move || handle_connection(stream, &app, &expected_token))
                {
                    log_server_error("request_thread_failed", &error.to_string());
                }
            }
        })
        .map_err(|error| format!("failed to start AgentLight loopback API: {error}"))?;

    Ok(())
}

fn handle_connection(mut stream: TcpStream, app: &AppHandle, expected_token: &str) {
    serve_connection(&mut stream, expected_token, |event| {
        codex::apply_event(app, event)
    });
}

fn serve_connection(
    stream: &mut TcpStream,
    expected_token: &str,
    apply_event: impl FnOnce(AgentEvent) -> Result<(), String>,
) {
    let response = match read_request(stream) {
        Ok(request) => {
            let path = request.path.clone();
            let result = route_with_handler(request, expected_token, apply_event);
            log_request(&path, result.status);
            result
        }
        Err(error) => {
            log_request("<malformed>", 400);
            ApiResponse::new(
                400,
                serde_json::json!({ "accepted": false, "error": error }),
            )
        }
    };

    let _ = stream.write_all(response.encode().as_bytes());
    let _ = stream.flush();
}

fn route_with_handler(
    request: HttpRequest,
    expected_token: &str,
    apply_event: impl FnOnce(AgentEvent) -> Result<(), String>,
) -> ApiResponse {
    if request.method == "GET" && request.path == HEALTH_PATH {
        return ApiResponse::new(200, serde_json::json!({ "status": "ok" }));
    }

    if request.method != "POST" || request.path != EVENT_PATH {
        return ApiResponse::new(
            404,
            serde_json::json!({ "accepted": false, "error": "not_found" }),
        );
    }

    if request.headers.get("content-type").is_none_or(|value| {
        !value
            .split(';')
            .next()
            .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case("application/json"))
    }) {
        return ApiResponse::new(
            400,
            serde_json::json!({ "accepted": false, "error": "invalid_content_type" }),
        );
    }

    let expected_authorization = format!("Bearer {expected_token}");
    if request
        .headers
        .get("authorization")
        .is_none_or(|value| value != &expected_authorization)
    {
        return ApiResponse::new(
            401,
            serde_json::json!({ "accepted": false, "error": "unauthorized" }),
        );
    }

    let event = match serde_json::from_slice::<AgentEvent>(&request.body) {
        Ok(event) => event,
        Err(_) => {
            return ApiResponse::new(
                400,
                serde_json::json!({ "accepted": false, "error": "invalid_event" }),
            )
        }
    };

    let event_id = event.event_id.clone();
    match apply_event(event) {
        Ok(()) => ApiResponse::new(
            200,
            serde_json::json!({ "accepted": true, "event_id": event_id }),
        ),
        Err(_) => ApiResponse::new(
            400,
            serde_json::json!({ "accepted": false, "error": "invalid_event" }),
        ),
    }
}

#[derive(Debug)]
struct ApiResponse {
    status: u16,
    body: serde_json::Value,
}

impl ApiResponse {
    fn new(status: u16, body: serde_json::Value) -> Self {
        Self { status, body }
    }

    fn encode(&self) -> String {
        let body = self.body.to_string();
        format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.status,
            status_text(self.status),
            body.len()
        )
    }
}

fn log_request(path: &str, status: u16) {
    let level = if status >= 500 {
        "ERROR"
    } else if status >= 400 {
        "WARN"
    } else {
        "INFO"
    };
    let route = match path {
        EVENT_PATH => EVENT_PATH,
        HEALTH_PATH => HEALTH_PATH,
        _ => "unknown",
    };
    eprintln!(
        "{}",
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "level": level,
            "event": "local_api_request",
            "route": route,
            "status": status,
        })
    );
}

fn log_server_error(event: &str, message: &str) {
    eprintln!(
        "{}",
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "level": "ERROR",
            "event": event,
            "message": message,
        })
    );
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, &'static str> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| "read_timeout")?;

    let mut buffer = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];
    let mut header_end;
    let mut content_length;

    loop {
        if buffer.len() > MAX_REQUEST_BYTES {
            return Err("request_too_large");
        }

        let read = stream.read(&mut chunk).map_err(|_| "read_failed")?;
        if read == 0 {
            return Err("unexpected_end");
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > MAX_REQUEST_BYTES {
            return Err("request_too_large");
        }

        let Some(found_header_end) = find_header_end(&buffer) else {
            continue;
        };

        header_end = found_header_end;
        let (request_line, headers) =
            parse_headers(&buffer[..header_end]).ok_or("invalid_headers")?;
        let method = request_line
            .split_whitespace()
            .next()
            .ok_or("invalid_request_line")?;
        content_length = match headers.get("content-length") {
            Some(value) => value
                .parse::<usize>()
                .map_err(|_| "invalid_content_length")?,
            None if method.eq_ignore_ascii_case("GET") => 0,
            None => return Err("missing_content_length"),
        };

        if content_length > MAX_REQUEST_BYTES {
            return Err("request_too_large");
        }
        if header_end + 4 + content_length > MAX_REQUEST_BYTES {
            return Err("request_too_large");
        }

        if buffer.len() >= header_end + 4 + content_length {
            break;
        }
    }

    let (request_line, headers) = parse_headers(&buffer[..header_end]).ok_or("invalid_headers")?;
    let body_start = header_end + 4;
    let body_end = body_start + content_length;
    let body = buffer[body_start..body_end].to_vec();
    let mut request_parts = request_line.split_whitespace();

    Ok(HttpRequest {
        method: request_parts
            .next()
            .ok_or("invalid_request_line")?
            .to_string(),
        path: request_parts
            .next()
            .ok_or("invalid_request_line")?
            .to_string(),
        headers,
        body,
    })
}

fn parse_headers(bytes: &[u8]) -> Option<(String, HashMap<String, String>)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?.to_string();
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect();

    Some((request_line, headers))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    use super::{
        read_request, route_with_handler, serve_connection, HttpRequest, EVENT_PATH, HEALTH_PATH,
    };

    const TOKEN: &str = "test-token";

    fn request(method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> HttpRequest {
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            headers: headers
                .iter()
                .map(|(name, value)| (name.to_ascii_lowercase(), value.to_string()))
                .collect(),
            body: body.as_bytes().to_vec(),
        }
    }

    fn valid_event() -> &'static str {
        r#"{"schema_version":1,"event_id":"evt-1","agent":"codex","session_id":"session-1","project":"agent-light","event":"agent_working","occurred_at":"2026-01-01T12:00:00Z"}"#
    }

    #[test]
    fn health_endpoint_is_minimal_and_does_not_require_authentication() {
        let response = route_with_handler(request("GET", HEALTH_PATH, &[], ""), TOKEN, |_| {
            panic!("health endpoint must not process events")
        });
        assert_eq!(response.status, 200);
        assert_eq!(response.body["status"], "ok");
    }

    #[test]
    fn event_endpoint_rejects_missing_or_wrong_token() {
        for authorization in [None, Some("Bearer wrong-token")] {
            let headers = match authorization {
                Some(value) => vec![
                    ("content-type", "application/json"),
                    ("authorization", value),
                ],
                None => vec![("content-type", "application/json")],
            };
            let response = route_with_handler(
                request("POST", EVENT_PATH, &headers, valid_event()),
                TOKEN,
                |_| panic!("unauthorized events must not be applied"),
            );
            assert_eq!(response.status, 401);
        }
    }

    #[test]
    fn malformed_payload_and_wrong_media_type_are_rejected() {
        let wrong_media_type = route_with_handler(
            request(
                "POST",
                EVENT_PATH,
                &[
                    ("content-type", "text/plain"),
                    ("authorization", "Bearer test-token"),
                ],
                valid_event(),
            ),
            TOKEN,
            |_| panic!("wrong media type must not process events"),
        );
        assert_eq!(wrong_media_type.status, 400);

        let malformed = route_with_handler(
            request(
                "POST",
                EVENT_PATH,
                &[
                    ("content-type", "application/json"),
                    ("authorization", "Bearer test-token"),
                ],
                "{not-json",
            ),
            TOKEN,
            |_| panic!("malformed events must not be applied"),
        );
        assert_eq!(malformed.status, 400);
    }

    #[test]
    fn valid_authenticated_event_is_passed_to_the_core() {
        let mut called = false;
        let response = route_with_handler(
            request(
                "POST",
                EVENT_PATH,
                &[
                    ("content-type", "Application/JSON; charset=utf-8"),
                    ("authorization", "Bearer test-token"),
                ],
                valid_event(),
            ),
            TOKEN,
            |event| {
                called = true;
                assert_eq!(event.event_id, "evt-1");
                Ok(())
            },
        );
        assert!(called);
        assert_eq!(response.status, 200);
        assert_eq!(response.body["accepted"], true);
        assert_eq!(response.body["event_id"], "evt-1");
    }

    #[test]
    fn rejected_core_event_returns_bad_request() {
        let response = route_with_handler(
            request(
                "POST",
                EVENT_PATH,
                &[
                    ("content-type", "application/json"),
                    ("authorization", "Bearer test-token"),
                ],
                valid_event(),
            ),
            TOKEN,
            |_| Err("rejected".to_string()),
        );
        assert_eq!(response.status, 400);
    }

    #[test]
    fn unknown_route_returns_not_found() {
        let response = route_with_handler(request("GET", "/unknown", &[], ""), TOKEN, |_| {
            panic!("unknown routes must not process events")
        });
        assert_eq!(response.status, 404);
    }

    #[test]
    fn request_reader_rejects_oversized_content_length() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        client
            .write_all(b"POST /api/v1/events HTTP/1.1\r\nContent-Length: 65537\r\n\r\n")
            .unwrap();
        client.flush().unwrap();

        assert_eq!(read_request(&mut server).unwrap_err(), "request_too_large");
    }

    #[test]
    fn health_get_without_content_length_is_valid() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        client
            .write_all(
                b"GET /api/v1/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        client.flush().unwrap();

        serve_connection(&mut server, TOKEN, |_| {
            panic!("health request must not process events")
        });
        drop(server);
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with(r#"{"status":"ok"}"#));
    }

    #[test]
    fn loopback_http_event_request_reaches_the_core_and_returns_json() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut applied = false;
            serve_connection(&mut stream, TOKEN, |event| {
                applied = true;
                assert_eq!(event.event_id, "evt-1");
                Ok(())
            });
            applied
        });

        let body = valid_event();
        let mut client = TcpStream::connect(address).unwrap();
        write!(
            client,
            "POST {EVENT_PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAuthorization: Bearer {TOKEN}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(server.join().unwrap());
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with(r#"{"accepted":true,"event_id":"evt-1"}"#));
    }

    #[test]
    fn response_encoder_sets_json_content_length_and_close() {
        let response = route_with_handler(request("GET", HEALTH_PATH, &[], ""), TOKEN, |_| {
            panic!("health endpoint must not process events")
        });
        let encoded = response.encode();
        let mut pieces = encoded.split("\r\n\r\n");
        let headers = pieces.next().unwrap();
        let body = pieces.next().unwrap();
        assert!(headers.contains("Content-Type: application/json"));
        assert!(headers.contains("Connection: close"));
        assert!(headers.contains(&format!("Content-Length: {}", body.len())));

        let mut reader = body.as_bytes();
        let mut decoded = String::new();
        reader.read_to_string(&mut decoded).unwrap();
        assert_eq!(decoded, r#"{"status":"ok"}"#);
    }
}
