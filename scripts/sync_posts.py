"""Sync Markdown posts into the blog API using PUT-by-slug (idempotent)."""

from __future__ import annotations

import argparse
import os
import re
import sys
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any

import httpx


def parse_scalar(value: str) -> Any:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        return value[1:-1]
    lowered = value.lower()
    if lowered in {"true", "false"}:
        return lowered == "true"
    if lowered in {"null", "none", "~", ""}:
        return None
    return value


def parse_front_matter(text: str, path: Path) -> tuple[dict[str, Any], str]:
    if not text.startswith("---\n"):
        return {}, text.strip()
    try:
        _, metadata, body = text.split("---", 2)
    except ValueError as error:
        raise ValueError(f"{path}: unterminated YAML front matter") from error

    fields: dict[str, Any] = {}
    for line_number, line in enumerate(metadata.strip().splitlines(), start=2):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition(":")
        if not separator or not re.fullmatch(r"[a-zA-Z_][a-zA-Z0-9_]*", key.strip()):
            raise ValueError(
                f"{path}:{line_number}: expected a simple key: value field"
            )
        fields[key.strip()] = parse_scalar(value)
    return fields, body.strip()


def post_from_markdown(path: Path) -> tuple[str, dict[str, Any]]:
    slug = path.stem
    if not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", slug):
        raise ValueError(
            f"{path}: filename stem must be a lowercase dash-separated slug"
        )

    metadata, content = parse_front_matter(path.read_text(encoding="utf-8"), path)
    title = metadata.get("title")
    if not title:
        heading = re.search(r"^#\s+(.+?)\s*#*\s*$", content, flags=re.MULTILINE)
        title = heading.group(1) if heading else slug.replace("-", " ").title()

    if not content:
        raise ValueError(f"{path}: Markdown body is empty")

    excerpt = metadata.get("excerpt")
    if not excerpt:
        paragraph = next(
            (
                line.strip()
                for line in content.splitlines()
                if line.strip() and not line.lstrip().startswith("#")
            ),
            "",
        )
        excerpt = paragraph[:280] or None

    post: dict[str, Any] = {
        "title": str(title).strip(),
        "content": content,
        "genre": metadata.get("genre"),
        "excerpt": excerpt,
        "published": metadata.get("published", True),
    }
    if "published_at" in metadata:
        published_at = metadata["published_at"]
        if isinstance(published_at, str):
            try:
                published_at = datetime.fromisoformat(
                    published_at.replace("Z", "+00:00")
                )
            except ValueError:
                try:
                    published_at = datetime.combine(
                        date.fromisoformat(published_at), datetime.min.time()
                    )
                except ValueError as error:
                    raise ValueError(
                        f"{path}: published_at must be an ISO date or datetime"
                    ) from error
        if isinstance(published_at, datetime):
            if published_at.tzinfo is None:
                published_at = published_at.replace(tzinfo=timezone.utc)
            post["published_at"] = published_at.isoformat()
        else:
            raise ValueError(f"{path}: published_at must be an ISO date or datetime")
    return slug, post


def sync_post(api_url: str, token: str, slug: str, post: dict[str, Any]) -> None:
    url = f"{api_url.rstrip('/')}/posts/{slug}"
    try:
        response = httpx.put(
            url,
            json=post,
            headers={"Authorization": f"Bearer {token}"},
            timeout=15,
        )
        response.raise_for_status()
    except httpx.HTTPStatusError as error:
        details = error.response.text
        raise RuntimeError(
            f"{slug}: API returned HTTP {error.response.status_code}: {details}"
        ) from error
    except httpx.RequestError as error:
        raise RuntimeError(f"{slug}: could not reach API: {error}") from error


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", nargs="?", type=Path, default=Path("posts"))
    parser.add_argument(
        "--api", default=os.getenv("BLOG_API_URL", "http://localhost:3210")
    )
    args = parser.parse_args()

    token = os.getenv("BLOG_SYNC_TOKEN")
    if not token:
        parser.error("set BLOG_SYNC_TOKEN to the same secret configured on the backend")
    if not args.directory.is_dir():
        parser.error(f"posts directory does not exist: {args.directory}")

    markdown_files = sorted(args.directory.glob("*.md"))
    if not markdown_files:
        print(f"No Markdown files found in {args.directory}")
        return 0

    try:
        for path in markdown_files:
            slug, post = post_from_markdown(path)
            sync_post(args.api, token, slug, post)
            print(f"Synced {path} -> {slug}")
    except (OSError, ValueError, RuntimeError) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
