# Deploy guide (xrenmaster.xyz)

This app is an Actix Web server that serves the static UI and a WebSocket endpoint at `/ws`.

## 1) Configure environment

Copy the example env file and adjust as needed:

```bash
cp .env.example .env
```

Recommended for open access (all origins):

```
ALLOWED_ORIGINS=*
```

If you want to restrict to your domain only:

```
ALLOWED_ORIGINS=xrenmaster.xyz,https://www.xrenmaster.xyz
```

History size (number of messages to keep for new users):

```
MAX_HISTORY=200
```

History storage (SQLite file):

```
HISTORY_DB_PATH=chat.db
```

Set it empty to disable persistence:

```
HISTORY_DB_PATH=
```

## 2) Build and run

```bash
cargo build --release
HOST=0.0.0.0 PORT=8080 .\target\release\Chat.exe
```

## 3) Nginx reverse proxy (WebSocket enabled)

Use the config at:

```
deploy/nginx/xrenmaster.conf
```

Quick install example (Linux):

```bash
sudo cp deploy/nginx/xrenmaster.conf /etc/nginx/sites-available/xrenmaster.conf
sudo ln -s /etc/nginx/sites-available/xrenmaster.conf /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

## 4) TLS (optional but recommended)

After you issue certs (for example, with Let's Encrypt), uncomment the HTTPS server block
in `deploy/nginx/xrenmaster.conf` and add the certificate paths.

## 5) systemd (optional)

Copy and enable the unit:

```bash
sudo mkdir -p /opt/chat
sudo cp -r target/release/Chat /opt/chat/target/release/Chat
sudo cp .env /opt/chat/.env
sudo cp deploy/systemd/chat.service /etc/systemd/system/chat.service
sudo systemctl daemon-reload
sudo systemctl enable --now chat.service
```

Make sure the paths in `deploy/systemd/chat.service` match your install location.

## 6) Certbot renew hook (optional)

If you use certbot, add a deploy hook so nginx reloads after renewals:

```bash
sudo cp deploy/letsencrypt/renewal-hook.sh /etc/letsencrypt/renewal-hooks/deploy/chat-nginx-reload.sh
sudo chmod +x /etc/letsencrypt/renewal-hooks/deploy/chat-nginx-reload.sh
```

## 7) Verify

- Open `https://xrenmaster.xyz` in two tabs and send messages.
- WebSocket connects at `wss://xrenmaster.xyz/ws` automatically via the client.
- Health check: `https://xrenmaster.xyz/health` should return `ok`.

## 8) One-command deploy helper (optional)

Use the helper script to build, copy, and restart:

```bash
chmod +x deploy/deploy.sh
APP_DIR=/opt/chat BIN_NAME=Chat ./deploy/deploy.sh
```
