use actix::prelude::*;
use actix_files::Files;
use actix_web::{http::header, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone)]
struct ChatState {
    tx: broadcast::Sender<ChatMessage>,
    history: Arc<RwLock<VecDeque<ChatMessage>>>,
    max_history: usize,
    storage: Option<Storage>,
}

#[derive(Clone)]
struct AllowedOrigins {
    allow_all: bool,
    list: Vec<String>,
}

#[derive(Clone)]
struct Storage {
    path: String,
}

impl Storage {
    fn init(&self) {
        if let Ok(conn) = Connection::open(&self.path) {
            let _ = conn.execute(
                "CREATE TABLE IF NOT EXISTS messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user TEXT NOT NULL,
                    text TEXT NOT NULL,
                    ts INTEGER NOT NULL
                )",
                [],
            );
        }
    }

    fn load_history(&self, max: usize) -> Vec<ChatMessage> {
        let conn = match Connection::open(&self.path) {
            Ok(conn) => conn,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare(
            "SELECT user, text, ts FROM messages ORDER BY id DESC LIMIT ?1",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };

        let mut items = Vec::new();
        if let Ok(rows) = stmt.query_map([max as i64], |row| {
            Ok(ChatMessage {
                user: row.get(0)?,
                text: row.get(1)?,
                ts: row.get(2)?,
            })
        }) {
            for row in rows.flatten() {
                items.push(row);
            }
        }
        items.reverse();
        items
    }

    fn insert(&self, msg: ChatMessage) {
        let path = self.path.clone();
        actix_web::rt::spawn(async move {
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(conn) = Connection::open(path) {
                    let _ = conn.execute(
                        "INSERT INTO messages (user, text, ts) VALUES (?1, ?2, ?3)",
                        params![msg.user, msg.text, msg.ts],
                    );
                }
            })
            .await;
        });
    }
}

impl AllowedOrigins {
    fn from_env() -> Self {
        let raw = env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "*".to_string());
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed == "*" {
            return Self {
                allow_all: true,
                list: Vec::new(),
            };
        }

        let list = trimmed
            .split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>();

        Self {
            allow_all: list.is_empty(),
            list,
        }
    }

    fn is_allowed(&self, origin: Option<&str>) -> bool {
        if self.allow_all {
            return true;
        }

        let origin = match origin {
            Some(value) => value,
            None => return false,
        };

        self.list.iter().any(|allowed| {
            if allowed.starts_with("http://") || allowed.starts_with("https://") {
                origin.eq_ignore_ascii_case(allowed)
            } else {
                origin.eq_ignore_ascii_case(&format!("https://{}", allowed))
                    || origin.eq_ignore_ascii_case(&format!("http://{}", allowed))
            }
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    user: String,
    text: String,
    ts: i64,
}

#[derive(Deserialize)]
struct InboundMessage {
    user: String,
    text: String,
}

struct WsSession {
    state: Arc<ChatState>,
    rx: Option<broadcast::Receiver<ChatMessage>>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let history = self.state.history.clone();
        ctx.spawn(
            async move {
                let guard = history.read().await;
                guard.iter().cloned().collect::<Vec<_>>()
            }
            .into_actor(self)
            .map(|messages, _, ctx| {
                for msg in messages {
                    if let Ok(text) = serde_json::to_string(&msg) {
                        ctx.text(text);
                    }
                }
            }),
        );

        if let Some(rx) = self.rx.take() {
            ctx.add_stream(BroadcastStream::new(rx));
        }
    }
}

impl StreamHandler<Result<ChatMessage, tokio_stream::wrappers::errors::BroadcastStreamRecvError>>
    for WsSession
{
    fn handle(
        &mut self,
        item: Result<ChatMessage, tokio_stream::wrappers::errors::BroadcastStreamRecvError>,
        ctx: &mut Self::Context,
    ) {
        if let Ok(msg) = item {
            if let Ok(text) = serde_json::to_string(&msg) {
                ctx.text(text);
            }
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Text(text)) => {
                if let Ok(inbound) = serde_json::from_str::<InboundMessage>(&text) {
                    let ts = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let msg = ChatMessage {
                        user: inbound.user,
                        text: inbound.text,
                        ts,
                    };
                    {
                        let history = self.state.history.clone();
                        let max = self.state.max_history;
                        let storage = self.state.storage.clone();
                        let msg_for_history = msg.clone();
                        actix_web::rt::spawn(async move {
                            let mut guard = history.write().await;
                            guard.push_back(msg_for_history.clone());
                            while guard.len() > max {
                                guard.pop_front();
                            }
                            drop(guard);
                            if let Some(storage) = storage {
                                storage.insert(msg_for_history);
                            }
                        });
                    }
                    let _ = self.state.tx.send(msg);
                }
            }
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {}
        }
    }
}

async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<Arc<ChatState>>,
    origins: web::Data<AllowedOrigins>,
) -> Result<HttpResponse, Error> {
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if !origins.is_allowed(origin) {
        return Ok(HttpResponse::Forbidden().finish());
    }

    let rx = state.tx.subscribe();
    let session = WsSession {
        state: state.get_ref().clone(),
        rx: Some(rx),
    };
    ws::start(session, &req, stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (tx, _) = broadcast::channel::<ChatMessage>(256);
    let max_history = env::var("MAX_HISTORY")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(200);
    let storage_path = env::var("HISTORY_DB_PATH").unwrap_or_else(|_| "chat.db".to_string());
    let storage = if storage_path.trim().is_empty() {
        None
    } else {
        let storage = Storage { path: storage_path };
        storage.init();
        Some(storage)
    };
    let initial_history = storage
        .as_ref()
        .map(|storage| storage.load_history(max_history))
        .unwrap_or_default();
    let state = Arc::new(ChatState {
        tx,
        history: Arc::new(RwLock::new(VecDeque::from(initial_history))),
        max_history,
        storage,
    });
    let origins = AllowedOrigins::from_env();
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .app_data(web::Data::new(origins.clone()))
            .route("/ws", web::get().to(ws_handler))
            .route("/health", web::get().to(|| async { HttpResponse::Ok().body("ok") }))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
