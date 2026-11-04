# Personal blog MVP

Minimal Axum + PostgreSQL backend. The frontend is intentionally untouched.

## API

- `GET /posts?genre=rust&limit=20&offset=0` lists published posts, newest first. Genre matching is case-insensitive; pagination is capped at 100.
- `GET /posts/{id}` returns a published post by its slug (the `.md` filename without its extension).
- `POST /posts/{id}` creates a post; `PUT /posts/{id}` creates or replaces one; `DELETE /posts/{id}` removes one. These write routes require `Authorization: Bearer $BLOG_SYNC_TOKEN`.
- `GET /health` checks the database connection.
- The existing `/auth/*` routes remain mounted and return `501 Not Implemented` until authentication is implemented.

Write requests use JSON with `title`, `content`, and optional `genre`, `excerpt`, `published`, and `published_at`. Posts are published by default. Slugs must be lowercase dash-separated words/numbers, for example `my-first-post`.

## Markdown sync

Put posts in a `posts/` directory at the repository root. Each `.md` filename stem is the post ID. Optional simple YAML front matter supports `title`, `genre`, `excerpt`, `published`, and ISO `published_at`. The Markdown body is saved as content; if absent, title is inferred from its first H1 or the slug and excerpt from its first paragraph. Example:

```markdown
---
title: My first post
genre: Rust
published_at: 2026-10-26
---

Post content goes here.
```

Configure `BLOG_SYNC_TOKEN` in the backend environment and use the same value when running `uv run python scripts/sync_posts.py`. The script defaults to `./posts` and `http://localhost:3210`; override those with a positional directory and `--api`, or set `BLOG_API_URL`. It PUTs every Markdown file, making repeated runs safe. It does not delete database posts when files disappear, to avoid accidental data loss.

The Python sync utility uses the project environment managed by uv; run `uv sync` once to create/update `.venv`, then run it with `uv run python scripts/sync_posts.py`.

For local backend development, start PostgreSQL with `docker compose -f compose.dev.yaml up -d`, set `DATABASE_URL=postgresql://blog:dev@localhost:5432/nyanko_blog` and `BLOG_SYNC_TOKEN` in the environment, then run the backend from `backend/` with `cargo run`. Migrations run automatically on startup. The server listens on port `3210` by default; set `BNS_PORT` to change it.
