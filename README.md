# delim-doctor

A stdio MCP server that scans source files for unbalanced brackets and delimiters (`()` `[]` `{}`), pinpointing the exact **line:col** of each problem.

It does not do full syntax parsing (that is rust-analyzer's job). It does one thing: a single-pass scan that reports every extra or missing delimiter with a context snippet, and can optionally apply conservative fixes to the cases whose repair is unambiguous.

## Features

- Reports **all** imbalance points, not just the first one.
- Locates each problem by line and column, with a context snippet.
- Skips strings and comments so delimiters inside them are ignored.
- Optional conservative fixing: deletes stray closers and appends missing closers at end-of-file; never guesses where the insertion point is ambiguous.
- Confines every file access to `--workspace-root` (defaults to cwd).

## Installation

### npm (recommended)

```sh
npm install -g delim-doctor
```

### From source

Requires a recent stable Rust toolchain (edition 2024).

```sh
cargo install delim-doctor
```

## Usage

The binary is a stdio MCP server. When `--workspace-root` is omitted, it defaults to the current working directory. All file access is confined to that directory.

```sh
# scan from current directory
delim-doctor

# scan a specific project
delim-doctor --workspace-root /path/to/project

# print version
delim-doctor --version
```

### MCP client setup

#### opencode

Add to the `mcp` section of `opencode.json`:

```json
{
  "mcp": {
    "delim-doctor": {
      "type": "local",
      "command": ["npx", "-y", "delim-doctor"]
    }
  }
}
```

#### Claude Desktop / Cursor / other clients

```json
{
  "mcpServers": {
    "delim-doctor": {
      "command": "npx",
      "args": ["-y", "delim-doctor"]
    }
  }
}
```

## Tools

### `delim_scan` (read-only)

Scans a file and reports all delimiter imbalance points.

| Parameter | Required | Description |
|---|---|---|
| `path` | yes | File path, absolute or relative to `--workspace-root` |
| `language` | no | `rust` or `generic`; defaults to extension-based detection |
| `max_problems` | no | Maximum number of problems to return, default 20 |
| `context_lines` | no | Context lines around each problem, default 1 |
| `cursor` | no | `next_cursor` from a previous response, for pagination |

Returns a two-block `CallToolResult`: a JSON content block (stable schema) plus a short text summary, also exposed as `structuredContent`.

JSON structure:

```json
{
  "schema_version": 1,
  "file": "/absolute/path/to/file.rs",
  "language": "rust",
  "total": 1,
  "truncated": false,
  "next_cursor": null,
  "problems": [
    {
      "kind": "missingClose",
      "ch": "{",
      "line": 42,
      "col": 5,
      "byte_offset": 812,
      "expected": "}",
      "snippet": " 40 | fn foo() {\n 41 |     let x = 1;\n     |         ^ missing }\n 42 |     let y = 2;",
      "depth_at": 2,
      "at_eof": false
    }
  ]
}
```

- `kind`: `missingClose` (opened but never closed) or `unexpectedClose` (extra closing delimiter).
- `line`: 1-based. `col`: 1-based Unicode scalar column (not a byte column, not display width, tabs are not expanded).
- `at_eof`: `true` only for `missingClose` problems left on the stack at end of scan, meaning the insertion point for the missing closer is unambiguously at end-of-file.

### `delim_fix` (write-capable)

Applies **conservative** fixes. Defaults to `dry_run`, reporting only and never touching the file.

| Parameter | Required | Description |
|---|---|---|
| `path` | yes | File path |
| `language` | no | Same as `delim_scan` |
| `dry_run` | no | Default `true`, report only; set to `false` to write the file |

Only two categories of unambiguous problems are fixed automatically:

- `unexpectedClose` → delete the stray closing delimiter.
- `at_eof` `missingClose` → append the missing closer at end-of-file, innermost first.

All other problems (opens crossed by mismatch recovery, etc.) have a non-unique insertion point and are listed under `skipped`, **reported but never changed**.

With `dry_run: false`, the file is first backed up to `<file>.bak`, then a temp file is written in the same directory and atomically renamed into place.

Return structure:

```json
{
  "schema_version": 1,
  "file": "/absolute/path/to/file.rs",
  "language": "rust",
  "dry_run": true,
  "applied": [{ "action": "insert", "ch": "}", "line": 1, "col": 11 }],
  "skipped": [],
  "changed": true
}
```

## Language support

- `rust`: line comments, block comments (nested), `"..."` strings with escapes, `r"..."` / `r#"..."#` raw strings, `b"..."` / `b'..'`, `br#..`, and the ambiguity between `'a` lifetimes and character literals.
- `generic`: `//`, `/* */`, `#` line comments, single- and double-quoted strings.
- Unknown extensions fall back to `generic` without error. Any other `language` value returns `invalid_params`.

## Design

Single pass, O(n) time and O(n) stack depth, reporting **all** imbalance points rather than just the first.

When a closing delimiter does not match the top of the stack, the scanner searches downward for the nearest matching open. If found, each open crossed over is reported as `missingClose` and the matching open is popped normally; if not found, the closer is reported as `unexpectedClose`. This deterministic recovery keeps every problem independently locatable and avoids a single mismatch cascading into false positives for the rest of the file.

## Security

- Every path must resolve inside `--workspace-root` (prefix check after canonicalization, blocking `..` and symlink escapes).
- Rejected: directories, device files (Windows `NUL`/`CON`/..., Unix `/dev/*`), null bytes, files over 10 MB, and files whose extension is not on the allow-list.
- `delim_fix` backs up before writing and replaces atomically via a temp file.

## Contributing

Issues and pull requests are welcome.

Before opening a pull request, make sure the checks pass:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## License

MIT License. See [LICENSE](LICENSE).
