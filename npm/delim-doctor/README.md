# delim-doctor (npm)

npm distribution shim for the Rust `delim-doctor` binary. This package ships no binary, only `bin/cli.js`: it locates the platform package for the current platform (`@rcoplolabs/delim-doctor-<platform>`) and forwards stdio to it unchanged.

Platform packages are installed through `optionalDependencies`; npm fetches only the one matching the current `os`/`cpu`/`libc`, with zero `postinstall`.

## Usage

```sh
npx delim-doctor --version
```

To use it as an MCP server, add to your MCP client config:

### opencode

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

### Claude Desktop / Cursor / other clients

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

`--workspace-root` is optional — defaults to the current working directory. MCP clients typically set CWD to the project root when spawning the server.

See the [source repository](https://github.com/rcoplolabs/delim-doctor) for full documentation.
