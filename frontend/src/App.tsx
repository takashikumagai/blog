import { useEffect, useRef, useState } from 'react'
import type { ComponentProps } from 'react'
import { Link, Route, Routes, useParams, useSearchParams } from 'react-router-dom'
import { Markdown } from '@tanstack/markdown/react'
import type { MarkdownComponents } from '@tanstack/markdown/react'
import './App.css'

type ThemePreference = 'light' | 'dark' | 'system'

type PostSummary = {
  id: string
  title: string
  genre: string | null
  excerpt: string | null
  published_at: string
}

type Post = PostSummary & {
  content: string
}

type PostPage = {
  items: PostSummary[]
  total: number
  limit: number
  offset: number
}

const PAGE_SIZE = 8
const API_BASE = '/api'
const themeOptions: { value: ThemePreference; label: string }[] = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
  { value: 'system', label: 'System' },
]

function formatDate(value: string) {
  return new Intl.DateTimeFormat('en', {
    month: 'long',
    day: 'numeric',
    year: 'numeric',
    timeZone: 'UTC',
  }).format(new Date(value))
}

function readTime(content: string) {
  const words = content.trim().split(/\s+/).filter(Boolean).length
  return `${Math.max(1, Math.ceil(words / 220))} min read`
}

function articleBody(content: string, title: string) {
  const heading = content.match(/^#\s+(.+?)\s*#*\s*(?:\r?\n|$)/)
  if (!heading || heading[1].trim() !== title.trim()) return content
  return content.slice(heading[0].length).trimStart()
}

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(
      response.status === 404
        ? 'This post could not be found.'
        : `The blog could not be reached (HTTP ${response.status}).`,
    )
  }
  return response.json() as Promise<T>
}

function Icon({ name, className = 'size-5' }: { name: string; className?: string }) {
  const common = {
    className,
    fill: 'none',
    stroke: 'currentColor',
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    strokeWidth: 1.7,
    viewBox: '0 0 24 24',
    'aria-hidden': true as const,
  }

  if (name === 'sun') {
    return (
      <svg {...common}>
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2m0 16v2M4.93 4.93l1.42 1.42m11.3 11.3 1.42 1.42M2 12h2m16 0h2M4.93 19.07l1.42-1.42m11.3-11.3 1.42-1.42" />
      </svg>
    )
  }
  if (name === 'moon') {
    return (
      <svg {...common}>
        <path d="M20.9 13A9 9 0 0 1 11 3.1 9 9 0 1 0 20.9 13Z" />
      </svg>
    )
  }
  if (name === 'monitor') {
    return (
      <svg {...common}>
        <rect x="3" y="4" width="18" height="13" rx="2" />
        <path d="M8 21h8m-4-4v4" />
      </svg>
    )
  }
  if (name === 'arrow') {
    return (
      <svg {...common}>
        <path d="M5 12h14m-6-6 6 6-6 6" />
      </svg>
    )
  }
  if (name === 'check') {
    return (
      <svg {...common}>
        <path d="m5 12 4 4L19 6" />
      </svg>
    )
  }
  return null
}

function ThemeSelector() {
  const [preference, setPreference] = useState<ThemePreference>(() => {
    const saved = localStorage.getItem('theme')
    return saved === 'light' || saved === 'dark' ? saved : 'system'
  })
  const [isOpen, setIsOpen] = useState(false)
  const menuRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)')
    const applyTheme = () => {
      const isDark = preference === 'dark' || (preference === 'system' && media.matches)
      document.documentElement.classList.toggle('dark', isDark)
      document.documentElement.style.colorScheme = isDark ? 'dark' : 'light'
    }

    applyTheme()
    media.addEventListener('change', applyTheme)
    return () => media.removeEventListener('change', applyTheme)
  }, [preference])

  useEffect(() => {
    if (!isOpen) return

    const closeOnOutsideClick = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) setIsOpen(false)
    }
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsOpen(false)
    }
    document.addEventListener('pointerdown', closeOnOutsideClick)
    document.addEventListener('keydown', closeOnEscape)
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsideClick)
      document.removeEventListener('keydown', closeOnEscape)
    }
  }, [isOpen])

  const currentIcon =
    preference === 'system'
      ? 'monitor'
      : preference === 'dark'
        ? 'moon'
        : 'sun'

  return (
    <div className="relative" ref={menuRef}>
      <button
        aria-label={`Color theme: ${preference}`}
        aria-expanded={isOpen}
        aria-haspopup="menu"
        className="theme-trigger"
        onClick={() => setIsOpen((open) => !open)}
        type="button"
      >
        <Icon name={currentIcon} className="size-[18px]" />
        <span className="hidden sm:inline">{preference}</span>
      </button>
      {isOpen && (
        <div className="theme-menu" role="menu" aria-label="Choose a color theme">
          <p className="theme-menu-label">Appearance</p>
          {themeOptions.map((option) => (
            <button
              aria-checked={preference === option.value}
              className="theme-option"
              key={option.value}
              onClick={() => {
                localStorage.setItem('theme', option.value)
                setPreference(option.value)
                setIsOpen(false)
              }}
              role="menuitemradio"
              type="button"
            >
              <Icon
                name={option.value === 'system' ? 'monitor' : option.value}
                className="size-[17px]"
              />
              <span>{option.label}</span>
              {preference === option.value && (
                <Icon name="check" className="ml-auto size-4" />
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

function SiteHeader() {
  return (
    <header className="site-header">
      <div className="page-width flex h-[76px] items-center justify-between">
        <Link aria-label="Nyanko Journal home" className="brand" to="/">
          <span className="brand-mark">n.</span>
          <span className="brand-name">nyanko<span className="brand-dot">.</span></span>
        </Link>
        <nav aria-label="Main navigation" className="flex items-center gap-5 sm:gap-8">
          <Link className="nav-link" to="/">Writing</Link>
          <a className="nav-link nav-link-muted" href="#about">About</a>
          <ThemeSelector />
        </nav>
      </div>
    </header>
  )
}

function SiteFooter() {
  return (
    <footer className="site-footer">
      <div className="page-width flex flex-col gap-2 py-7 text-sm sm:flex-row sm:items-center sm:justify-between">
        <p>nyanko journal</p>
        <p id="about">A small collection of notes from everyday life.</p>
      </div>
    </footer>
  )
}

function PostListPage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const query = searchParams.toString()
  const currentPage = Math.max(1, Number(searchParams.get('page')) || 1)
  const selectedGenre = searchParams.get('genre')
  const [retry, setRetry] = useState(0)
  const [result, setResult] = useState<{
    key: string
    posts?: PostPage
    error?: string
  }>({ key: '' })
  const params = new URLSearchParams(query)
  params.set('limit', String(PAGE_SIZE))
  params.set('offset', String((currentPage - 1) * PAGE_SIZE))
  params.delete('page')
  const requestQuery = params.toString()
  const requestKey = `${requestQuery}:${retry}`
  const loading = result.key !== requestKey
  const posts = loading ? null : result.posts ?? null
  const error = loading ? null : result.error ?? null

  useEffect(() => {
    let active = true
    fetchJson<PostPage>(`${API_BASE}/posts?${requestQuery}`)
      .then((result) => {
        if (active) setResult({ key: requestKey, posts: result })
      })
      .catch((cause: unknown) => {
        if (active) {
          setResult({
            key: requestKey,
            error: cause instanceof Error ? cause.message : 'Something went wrong while loading posts.',
          })
        }
      })
    return () => {
      active = false
    }
  }, [requestKey, requestQuery])

  const genres = [...new Set(posts?.items.map((post) => post.genre).filter((genre): genre is string => Boolean(genre)) ?? [])]
  const pageCount = posts ? Math.ceil(posts.total / PAGE_SIZE) : 0
  const updatePage = (page: number) => {
    const next = new URLSearchParams(searchParams)
    if (page === 1) next.delete('page')
    else next.set('page', String(page))
    setSearchParams(next)
  }

  return (
    <>
      <section className="page-width hero-section">
        <div className="hero-copy">
          <p className="eyebrow"><span className="eyebrow-rule" /> Notes on things that matter</p>
          <h1>Ideas worth <span className="serif-italic">keeping.</span></h1>
          <p className="hero-description">
            Thoughts on technology, the places we call home, and everything in between.
          </p>
        </div>
        <div aria-hidden="true" className="hero-stamp">
          <span>EST.</span>
          <strong>20<br />26</strong>
          <span>JOURNAL</span>
        </div>
      </section>

      <main className="page-width content-section">
        <div className="section-heading">
          <div>
            <p className="eyebrow">The journal</p>
            <h2>{selectedGenre ? selectedGenre : 'Latest writing'}</h2>
          </div>
          {posts && (
            <p className="post-count">
              {posts.total} {posts.total === 1 ? 'story' : 'stories'}
            </p>
          )}
        </div>

        {(genres.length > 0 || selectedGenre) && (
          <div aria-label="Filter by topic" className="genre-list">
            <button
              aria-pressed={!selectedGenre}
              className={`genre-chip${!selectedGenre ? ' genre-chip-active' : ''}`}
              onClick={() => setSearchParams({})}
              type="button"
            >
              All writing
            </button>
            {genres.map((genre) => (
              <button
                aria-pressed={selectedGenre === genre}
                className={`genre-chip${selectedGenre === genre ? ' genre-chip-active' : ''}`}
                key={genre}
                onClick={() => setSearchParams({ genre })}
                type="button"
              >
                {genre}
              </button>
            ))}
          </div>
        )}

        {loading && (
          <div aria-label="Loading posts" className="post-list">
            {[0, 1, 2].map((item) => (
              <div aria-hidden="true" className="post-skeleton" key={item}>
                <div className="skeleton-line skeleton-meta" />
                <div className="skeleton-line skeleton-title" />
                <div className="skeleton-line skeleton-excerpt" />
              </div>
            ))}
          </div>
        )}

        {!loading && error && (
          <div className="state-card" role="alert">
            <span className="state-icon">!</span>
            <h3>We couldn’t load the journal</h3>
            <p>{error}</p>
            <button className="text-button" onClick={() => setRetry((count) => count + 1)} type="button">
              Try again <Icon name="arrow" className="size-4" />
            </button>
          </div>
        )}

        {!loading && !error && posts?.items.length === 0 && (
          <div className="state-card">
            <span aria-hidden="true" className="empty-flower">✳</span>
            <h3>{selectedGenre ? 'No stories in this topic yet.' : 'The first story is on its way.'}</h3>
            <p>
              {selectedGenre
                ? 'Try another topic, or come back to browse all writing.'
                : 'There’s nothing here just yet. Check back soon for something new to read.'}
            </p>
            {selectedGenre && (
              <button className="text-button" onClick={() => setSearchParams({})} type="button">
                Browse all writing <Icon name="arrow" className="size-4" />
              </button>
            )}
          </div>
        )}

        {!loading && !error && posts && posts.items.length > 0 && (
          <>
            <div className="post-list">
              {posts.items.map((post, index) => (
                <article className={`post-card${index === 0 && currentPage === 1 ? ' post-card-featured' : ''}`} key={post.id}>
                  <div className="post-card-meta">
                    <time dateTime={post.published_at}>{formatDate(post.published_at)}</time>
                    {post.genre && <span className="post-tag">{post.genre}</span>}
                  </div>
                  <h3>
                    <Link to={`/posts/${encodeURIComponent(post.id)}`}>{post.title}</Link>
                  </h3>
                  <p className="post-excerpt">
                    {post.excerpt || 'A story from the journal. Open to read the full post.'}
                  </p>
                  <Link aria-label={`Read ${post.title}`} className="read-link" to={`/posts/${encodeURIComponent(post.id)}`}>
                    Read story <Icon name="arrow" className="size-4" />
                  </Link>
                </article>
              ))}
            </div>
            {pageCount > 1 && (
              <nav aria-label="Post pages" className="pagination">
                <button disabled={currentPage <= 1} onClick={() => updatePage(currentPage - 1)} type="button">
                  Previous
                </button>
                <span>Page {currentPage} of {pageCount}</span>
                <button disabled={currentPage >= pageCount} onClick={() => updatePage(currentPage + 1)} type="button">
                  Next <Icon name="arrow" className="size-4" />
                </button>
              </nav>
            )}
          </>
        )}
      </main>
    </>
  )
}

const markdownComponents = {
  a(props: ComponentProps<'a'>) {
    const external = /^https?:\/\//i.test(props.href || '')
    return (
      <a
        {...props}
        rel={external ? 'noopener noreferrer' : props.rel}
        target={external ? '_blank' : props.target}
      />
    )
  },
} satisfies MarkdownComponents

function PostDetailPage() {
  const { id = '' } = useParams()
  const [result, setResult] = useState<{
    id: string
    post?: Post
    error?: string
  }>({ id: '' })
  const loading = result.id !== id
  const post = loading ? null : result.post ?? null
  const error = loading ? null : result.error ?? null

  useEffect(() => {
    let active = true
    fetchJson<Post>(`${API_BASE}/posts/${encodeURIComponent(id)}`)
      .then((result) => {
        if (active) setResult({ id, post: result })
      })
      .catch((cause: unknown) => {
        if (active) {
          setResult({
            id,
            error: cause instanceof Error ? cause.message : 'Something went wrong while loading this post.',
          })
        }
      })
    return () => {
      active = false
    }
  }, [id])

  return (
    <main className="page-width article-page">
      <Link className="back-link" to="/">
        <span aria-hidden="true">←</span> All writing
      </Link>
      {loading && (
        <div aria-label="Loading post" className="article-loading">
          <div className="skeleton-line skeleton-meta" />
          <div className="skeleton-line skeleton-article-title" />
          <div className="skeleton-line skeleton-article-title skeleton-short" />
          <div className="skeleton-line skeleton-excerpt" />
        </div>
      )}
      {!loading && error && (
        <div className="state-card article-state" role="alert">
          <span className="state-icon">!</span>
          <h3>{error === 'This post could not be found.' ? 'Story not found' : 'We couldn’t load this story'}</h3>
          <p>{error}</p>
          <Link className="text-button" to="/">Return to all writing <Icon name="arrow" className="size-4" /></Link>
        </div>
      )}
      {!loading && !error && post && (
        <article className="article">
          <header className="article-header">
            <div className="post-card-meta">
              <time dateTime={post.published_at}>{formatDate(post.published_at)}</time>
              {post.genre && <span className="post-tag">{post.genre}</span>}
              <span>{readTime(post.content)}</span>
            </div>
            <h1>{post.title}</h1>
            {post.excerpt && <p className="article-deck">{post.excerpt}</p>}
            <div className="article-byline">
              <span aria-hidden="true" className="author-mark">n.</span>
              <span>From the nyanko journal</span>
            </div>
          </header>
          <div className="markdown-body">
            <Markdown components={markdownComponents}>
              {articleBody(post.content, post.title)}
            </Markdown>
          </div>
          <div className="article-end">
            <span>—</span>
            <p>Thanks for reading.</p>
            <Link className="text-button" to="/">Explore more writing <Icon name="arrow" className="size-4" /></Link>
          </div>
        </article>
      )}
    </main>
  )
}

function App() {
  return (
    <div className="min-h-screen">
      <SiteHeader />
      <Routes>
        <Route element={<PostListPage />} path="/" />
        <Route element={<PostDetailPage />} path="/posts/:id" />
        <Route element={<NotFoundPage />} path="*" />
      </Routes>
      <SiteFooter />
    </div>
  )
}

function NotFoundPage() {
  return (
    <main className="page-width article-page">
      <div className="state-card article-state">
        <span className="state-icon">404</span>
        <h3>That page wandered off.</h3>
        <p>The page you’re looking for doesn’t exist.</p>
        <Link className="text-button" to="/">Back to the journal <Icon name="arrow" className="size-4" /></Link>
      </div>
    </main>
  )
}

export default App
