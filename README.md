# AI-Based Website

🚀 **Unique homepages generated at runtime by AI.**

This project is a Rust web application that dynamically generates full HTML5 landing pages using large language models (via [Groq](https://groq.com)). Every visit produces a fresh page, with ephemeral storage and automatic cleanup. No two requests are guaranteed to be the same.

---

## ✨ Features

- **AI-generated pages**  
  Pages are generated in real-time using different AI models (Groq API).

- **Ephemeral storage**  
  Recently generated pages are kept in a small in-memory bucket for reuse, then expired and cleaned up automatically.

- **Per-user experience**  
  Pages are tracked per user via cookies. You won’t be served the same cached page twice until you’ve cycled through the bucket.

- **Rate limiting**  
  Requests are limited per user per minute to prevent abuse.

- **Lightweight and fast**  
  Built with [Axum](https://github.com/tokio-rs/axum) + [Tokio](https://tokio.rs), no database required.

- **Telemetry built-in**  
  Structured logs with [tracing](https://docs.rs/tracing).

---

## ⚙️ Configuration

The app reads its configuration from environment variables (with `.env` support):

- `GROQ_API_KEY` – **required**, your Groq API key
- `RATE_LIMIT_PER_MINUTE` – optional, defaults to `10`
- `PORT` – optional, defaults to `8080`

Example `.env`:

```env
GROQ_API_KEY=sk-xxxxxxxxxxxxxxxx
RATE_LIMIT_PER_MINUTE=15
PORT=3000
````

---

## ▶️ Running

1. Install [Rust](https://www.rust-lang.org/tools/install) (latest stable recommended).
2. Clone this repo and enter the project directory.
3. Set your `.env` as described above.
4. Run:

```bash
cargo run --release
```

By default, the server starts at:
👉 `http://localhost:8080`

---

## 🔍 How It Works

1. **Request arrives**

   * User ID cookie is issued if not present.
   * Rate limiter checks quota.

2. **Serve or generate**

   * If there’s a cached unseen page, serve it.
   * Otherwise, generate a new page with Groq, cache it, and serve.

3. **Housekeeping**

   * Pages expire after \~60s if seen at least once.
   * Storage bucket is trimmed to a max size.

---

## 📜 License

MIT License © [lazar](https://bylazar.com)

---

## 🌐 Links

* Author: [bylazar.com](https://bylazar.com)
* Project GitHub: [lazarcloud/ai-based-website](https://github.com/lazarcloud/ai-based-website)