# HTTP serve mode

`agent-browser serve` runs a foreground Salvo HTTP server. CLI commands map to
`/api/...` routes with **typed parameters** and **correct HTTP verbs**. This is
**not** the observability dashboard (`dashboard start`).

## Start

```bash
agent-browser serve
agent-browser serve --host 127.0.0.1 --port 6578
agent-browser serve --cors http://localhost:3000   # repeat --cors; default is *
```

## Discovery

- Health: `GET /api/health`
- OpenAPI: `GET /openapi.json`
- Swagger UI: `GET /swagger-ui/` (typed params per command)

## Not exposed over REST

Use the CLI for these:

- `install` (downloads Chrome; interactive progress)
- `serve` (this server itself)
- `dashboard` (observability UI)
- `session` (rename/list session names on disk)
- `doctor` (diagnostics; use `agent-browser doctor` on the host)
- `upgrade` (self-update; use `agent-browser upgrade` on the host)

## Typed session routes (1:1 with CLI)

Each catalog command has a dedicated route in `HTTP_ROUTES` (`cli/src/commands/http_routes.rs`):

- **GET** — read-only (`get/text`, `snapshot`, `cookies`, `network/requests`, …)
- **DELETE** — destructive/clear (`close`, `cookies/clear`, `storage/local/clear`, …)
- **POST** — actions and mutations (`click`, `fill`, `goto`, …)

**Parameters**

- On **GET/DELETE**: command args as **query** parameters (`selector`, `text`, `url`, flags like `interactive`, …)
- On **POST**: same fields in a **JSON body** (optional `{}` when no args)
- **Global CLI flags** (`json`, `headed`, `sessionName`, `proxy`, …): always query string on any verb

Swagger lists each field with the correct type (string, integer, boolean, array).

Examples (session = `default`):

```bash
# GET read
curl 'http://127.0.0.1:6578/api/sessions/default/get/text?selector=%40e1'
curl 'http://127.0.0.1:6578/api/sessions/default/snapshot?interactive&compact'

# POST action
curl -X POST http://127.0.0.1:6578/api/sessions/default/click \
  -H 'Content-Type: application/json' -d '{"selector":"@e1"}'

# DELETE lifecycle
curl -X DELETE http://127.0.0.1:6578/api/sessions/default/close
```

## Escape hatches

- `POST /api/execute` and `POST /api/sessions/{session}/execute` — raw `args` argv list (any command)
- `GET|POST|DELETE /api/sessions/{session}/{**rest}` — fallback for dynamic paths (`tab/3`, `frame/foo`) not in the catalog

## Meta

- `GET /api/health`, `GET /api/sessions`, `DELETE /api/sessions`
- `GET /api/profiles`, `GET /api/skills`, `GET /api/skills/{name}`
- `POST /api/chat`, `GET /api/chat/status`, `GET /api/models`

## Batch SSE

```bash
curl -N -X POST http://127.0.0.1:6578/api/sessions/default/batch \
  -H 'Content-Type: application/json' \
  -d '{"stream":true,"commands":[["open","https://example.com"],["snapshot","-i"]]}'
```

## Chat

`POST /api/chat` with `{"session":"default","message":"...","stream":false}` when
`AI_GATEWAY_API_KEY` is set.
