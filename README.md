# Rust WebSocket Chat

Simple Actix Web chat server with a static UI and a WebSocket endpoint.

## Features

- Broadcast chat with WebSocket (`/ws`)
- Static UI served from `/`
- Health check endpoint (`/health`)
- Configurable host/port and allowed origins
- Persistent history (SQLite)
- Nginx + systemd deployment helpers

## Quick start

```bash
cargo run
```

Open `http://localhost:8080` in two tabs and send messages.

## Configuration

Environment variables (see `.env.example`):

- `HOST` (default: `0.0.0.0`)
- `PORT` (default: `8080`)
- `ALLOWED_ORIGINS` (default: `*`)
- `MAX_HISTORY` (default: `200`)
- `HISTORY_DB_PATH` (default: `chat.db`, set empty to disable persistence)

`ALLOWED_ORIGINS` can be `*` or a comma-separated list:

```
ALLOWED_ORIGINS=xrenmaster.xyz,https://www.xrenmaster.xyz
```

## Production

See `DEPLOY.md` for:

- Nginx reverse proxy (WebSocket-ready)
- HTTPS setup (Let's Encrypt paths)
- systemd service
- certbot renew hook
- one-command deploy script

## Persistence

History is stored in SQLite at `HISTORY_DB_PATH`. On startup, the last
`MAX_HISTORY` messages are loaded and shown to new users.

## Endpoints

- `GET /` UI
- `GET /ws` WebSocket
- `GET /health` health check
