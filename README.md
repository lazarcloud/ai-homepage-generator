# AI-Based Website

Unique homepages generated at runtime by AI.

This is a Rust web application that dynamically generates full HTML5 landing pages using large language models via the Groq API. Each visit can produce a fresh page, with ephemeral storage and automatic cleanup.

---

## Features

- AI-generated pages: created in real time via Groq.
- Ephemeral storage: small in-memory cache with TTL and cleanup.
- Per-user experience: cookie-based tracking to avoid repeats.
- Rate limiting: per-user, per-minute guard.
- Lightweight: Axum + Tokio, no database.
- Telemetry: structured logs using tracing.

---

## Configuration

The app reads from environment variables (with `.env` support):

- `GROQ_API_KEY`: required, your Groq API key
- `RATE_LIMIT_PER_MINUTE`: optional, default `10`
- `PORT`: optional, default `8080`

Example `.env`:

```env
GROQ_API_KEY=sk-xxxxxxxxxxxxxxxx
RATE_LIMIT_PER_MINUTE=15
PORT=3000
```

---

## Running

1. Install Rust (latest stable).
2. Clone this repo and cd into it.
3. Create `.env` as above.
4. Run:

```bash
cargo run --release
```

Default address: `http://localhost:8080`

---

## How It Works

1. Request arrives: cookie set if missing, rate limit checked.
2. Serve or generate: serve unseen cached page or generate a new one.
3. Housekeeping: pages expire (~60s if seen), bucket trimmed.

---

## License

MIT © [lazar](https://bylazar.com)

---

## Links

- Author: https://bylazar.com
- GitHub: https://github.com/lazarcloud/ai-based-website
