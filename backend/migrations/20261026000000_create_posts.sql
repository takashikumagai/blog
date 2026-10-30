CREATE TABLE posts (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    genre TEXT,
    excerpt TEXT,
    published BOOLEAN NOT NULL DEFAULT TRUE,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT posts_id_slug CHECK (
        length(id) BETWEEN 1 AND 200
        AND id ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
    )
);

CREATE INDEX posts_published_date_idx ON posts (published_at DESC, id)
    WHERE published = TRUE;
CREATE INDEX posts_genre_idx ON posts (LOWER(genre))
    WHERE published = TRUE;