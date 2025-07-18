# MCP Protocol Specification

This document describes the binary framing and JSON‑RPC message structure used by the Unity MCP server and clients.

## Frame Format

Each message is transmitted as a 4‑byte little‑endian length prefix followed by a UTF‑8 encoded JSON‑RPC payload. The length field indicates the number of bytes that make up the JSON document.

```
+---------+----------------------+
| length  | JSON‑RPC payload     |
+---------+----------------------+
  u32 LE       length bytes
```

A receiver must read exactly `length` bytes after the prefix to obtain a complete JSON‑RPC message. If fewer bytes are available the frame is considered **truncated** and should result in a protocol error.

## JSON‑RPC 2.0 Fields

All payloads conform to the [JSON‑RPC 2.0](https://www.jsonrpc.org/specification) specification with the following field requirements:

- `jsonrpc` — always the string `"2.0"`.
- `id` — string or number identifying the request. Required in requests and echoed in responses.
- `method` — RPC method name for requests. Not present in responses.
- `params` — object or array containing request parameters. `null` when a method has no parameters.
- `result` — returned on success. Only present in responses.
- `error` — object with `code`, `message` and optional `data`. Only present on failure.

A response must contain either `result` **or** `error`, but never both.

## Error Codes

| Code  | Applies to                | Meaning                          |
|------:|---------------------------|----------------------------------|
| -32600 | any                       | Invalid JSON‑RPC request         |
| -32601 | any                       | Method not found                 |
| -32602 | any                       | Invalid params                   |
| -32603 | any                       | Internal error                   |
| 1000   | `session.authenticate`    | Authentication failed            |
| 1001   | `transport.healthCheck`   | Health check failed              |

Custom codes starting at `1000` are reserved for application‑level errors.

## Examples

### `session.authenticate`

Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "session.authenticate",
  "params": { "token": "SECRET" }
}
```

Successful response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { "sessionId": "abc123" }
}
```

Failure response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": { "code": 1000, "message": "Authentication failed" }
}
```

### `transport.healthCheck`

Request

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "transport.healthCheck",
  "params": null
}
```

Response

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": { "status": "ok" }
}
```

