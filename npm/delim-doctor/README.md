# delim-doctor (npm)

npm distribution shim for the Rust `delim-doctor` binary. This package ships no binary, only `bin/cli.js`: it locates the platform package for the current platform (`@rcoplolabs/delim-doctor-<platform>`) and forwards stdio to it unchanged.

Platform packages are installed through `optionalDependencies`; npm fetches only the one matching the current `os`/`cpu`/`libc`, with zero `postinstall`.

## Usage

```sh
npx delim-doctor --version
```

To use it as an MCP server, pass `--workspace-root`:

```json
{
  "mcp": {
    "delim-doctor": {
      "type": "local",
      "command": ["npx", "delim-doctor", "--workspace-root", "/path/to/project"]
    }
  }
}
```

See the source repository README for full documentation.
