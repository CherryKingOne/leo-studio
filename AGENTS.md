# AGENTS.md

Leo Studio: a Rust (edition 2024) microkernel audiobook/production workbench. `core` = AST + Context + plugin SPI; `plugins/<domain>/registry` = registry layers; `plugins/<domain>/<name>` = driver subplugins; `plugins/interfaces/{api,mcp,cli}` = transports; `plugins/catalog` = the single compile-time plugin list; `app` = host binary.

## Commands

```bash
cargo run -p leo-app                                   # host: REST API + MCP on 127.0.0.1:58685
cargo run -p leo-plugin-cli -- import 测试.txt          # CLI binary is `leo-cli` (import/list/export/plugins/voices/synthesize)
cargo test --workspace                                 # all tests
cargo test -p leo-app                                  # host + HTTP/MCP integration tests
cargo test -p leo-core                                 # kernel + pipeline
cargo test -p leo-plugin-tts-edge                      # one plugin
cargo fmt && cargo clippy --workspace                  # style gate
```

Run a single test with `cargo test -p <crate> <name substring>`.

## Architecture rules (easy to get wrong)

- Adding a plugin must NOT touch `core/` or `plugins/interfaces/*`. Only: new crate under `plugins/<domain>/`, add it to root `Cargo.toml` `members`, add a dep + a `CATALOG` entry in `plugins/catalog`. Registry layers are `required: true` and cannot be disabled.
- Dependency direction: `app`/`cli` → `catalog` → registries + subplugins → `core`. Subplugins never import each other; transports know no concrete subplugin. Use `ctx.register::<dyn Trait>(id, ..)` and resolve via the domain `*Hub` (e.g. `TtsHub::of(ctx)`).
- Driver ids are strings; `"edge:zh-CN-XiaoxiaoNeural"` routes to a driver, no prefix uses the configured default.
- Known debt: API v2 project flow still uses its own copied text code + `Store` in `plugins/interfaces/api/src/{text,storage}`, not the registry hubs. Don't assume all I/O goes through hubs.

## Config & secrets

- Runtime config is `leo-plugins.json` in CWD, or a path in `LEO_PLUGINS_CONFIG` (env wins over file). Unknown fields are rejected (`deny_unknown_fields`) — a typo fails startup. `settings` is keyed by plugin id and validated by each subplugin.
- API keys come ONLY from env (`STEPFUN_API_KEY`, `MINIMAX_API_KEY`); never put them in `leo-plugins.json`. Missing key = plugin loads but registers no driver.
- Copy `.env.example` → `.env` (loaded by `dotenvy`). Defaults: port `58685`, CSV data in `.leo-studio/`, uploads in `storage/uploads/`.
- macOS builds rely on `.cargo/config.toml` pinning `DEVELOPER_DIR`/`SDKROOT` to CommandLineTools; if linking fails, check that SDK path exists.

## Testing gotchas

- Integration tests drive the router with `tower::ServiceExt::oneshot` and tempdirs — no external services.
- stepfun/minimax tests start a fake server on `127.0.0.1`. If `HTTP(S)_PROXY` is set, also set `NO_PROXY=127.0.0.1,localhost` or requests get proxied (404).
- Real TTS API tests are `#[ignore]` and cost money: `cargo test -p leo-plugin-tts-stepfun -- --ignored`.

## Stale files — do not follow

- `app/STRUCTURE.md` and `.claude/rules/code-style.md` describe the OLD Python/FastAPI layout (`app/modules`, `app/router.py`, pytest). The code is now Rust under `plugins/`. Trust the code and `docs/plugins/registry-layer.md`.
- `Cargo.lock` is gitignored but present locally; `app/Cargo.lock` is a leftover.

## Conventions

- No leading underscore in function/type names (enforced convention).
- `cargo fmt` + `cargo clippy` before finishing; errors use `AppError`/`CoreError`, no `unwrap` on request paths.