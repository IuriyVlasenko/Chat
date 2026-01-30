use actix::prelude::*;
use actix_files::Files;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Clone)]
struct ChatState {
    tx: broadcast::Sender<ChatMessage>,
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
    tx: broadcast::Sender<ChatMessage>,
    rx: Option<broadcast::Receiver<ChatMessage>>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
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
                    let _ = self.tx.send(msg);
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
) -> Result<HttpResponse, Error> {
    let rx = state.tx.subscribe();
    let session = WsSession {
        tx: state.tx.clone(),
        rx: Some(rx),
    };
    ws::start(session, &req, stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (tx, _) = broadcast::channel::<ChatMessage>(256);
    let state = Arc::new(ChatState { tx });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .route("/ws", web::get().to(ws_handler))
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}