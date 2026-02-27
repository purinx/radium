# radium

A minimal HTML rendering engine written in Rust.
Renders a local HTML document in a native GUI window.

## Usage

```
radium <directory>
```

The directory must contain an `index.html` file.
Assets (images, etc.) are resolved relative to the directory.

```sh
cargo run -- ./my-site

# Run the bundled sample
cargo run -- examples/sample
```

## Build

```sh
cargo build --release
```

## Keyboard

| Key | Action |
|-----|--------|
| `↓` | Scroll down |
| `↑` | Scroll up |
| `PageDown` / `Space` | Scroll down one page |
| `PageUp` | Scroll up one page |
| `Home` | Jump to top |
| `End` | Jump to bottom |

Mouse wheel scrolling is also supported.

## Supported HTML

### Structure

| Element | Behaviour |
|---------|-----------|
| `html`, `body`, `div`, `section`, `article`, `main`, `header`, `footer` | Transparent container — children rendered as-is |
| `head`, `title`, `script`, `style`, `meta`, `link` | Skipped entirely |

Unknown tags are treated as transparent containers.

### Headings

| Element | Font size | Margin top | Margin bottom |
|---------|----------:|-----------:|--------------:|
| `h1` | 32px | 24px | 16px |
| `h2` | 24px | 20px | 12px |
| `h3` | 20px | 16px | 8px |

All headings are rendered bold. `h4`–`h6` are treated as transparent containers.

### Text

| Element | Behaviour |
|---------|-----------|
| `p` | Block with 16px bottom margin |
| `strong` | Bold |
| `em` | Italic |
| `a` | Blue (`#0000EE`) with underline |
| `span` | No style change |

### Lists

| Element | Behaviour |
|---------|-----------|
| `ul` | Unordered list |
| `ol` | Ordered list |
| `li` | List item |

Lists have 8px top and bottom margins. Nesting is supported to any depth.

Bullet markers by nesting depth:

| Depth | Marker |
|------:|--------|
| 1 | `•` |
| 2 | `◦` |
| 3+ | `▪` |

Ordered list markers use the format `1.`, `2.`, `3.` …

### Void elements

| Element | Behaviour |
|---------|-----------|
| `br` | Line break |
| `hr` | Horizontal rule |
| `img` | Displays a local image (see below) |

Self-closing syntax (`/>`) is supported for all void elements.

### Images

`<img src="...">` loads a file relative to the site directory.
Network URLs are not supported.

- Supported formats: PNG, JPEG
- Images wider than the content area are scaled down proportionally.
- The `src` attribute is the only attribute read; all others are ignored.

## Not Supported

- CSS (external, inline, or `<style>` tags)
- `h4`–`h6`
- Form controls (`input`, `button`, `select`, etc.)
- Tables (`table`, `tr`, `td`, etc.)
- HTML entities (`&amp;`, `&lt;`, etc.)
- `class`, `id`, `href`, `data-*` and all other attributes (except `img src`)
- JavaScript
- Text wrapping / word wrap
- Network resources

## Specification

See [docs/spec/html-spec.md](docs/spec/html-spec.md) for the full HTML specification.
