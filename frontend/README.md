# Nyanko Journal frontend

Responsive React frontend for reading published blog posts.

## Development

From this directory, install dependencies and start Vite:

```sh
bun install
bun run dev
```

The Vite development server proxies `/api/*` requests to the backend at
`http://localhost:3210`, rewriting the `/api` prefix. Start the backend
separately. The prefix keeps API requests distinct from the frontend's
`/posts/:id` article routes.

## Reading experience

- `/` lists published posts, with topic filters and pagination.
- `/posts/:id` displays an individual post rendered with TanStack Markdown.
- The appearance menu supports light, dark, and system themes; the selection is
  saved in local storage.

The MVP is intentionally read-only. Posts continue to be created by the
repository's Markdown sync utility.

## Checks

```sh
bun run lint
bun run build
```
