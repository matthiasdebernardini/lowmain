# lowmain — Agent-First Neo4j CLI

Built on `agcli` v0.7.0. Wok-themed play on "lo mein."

## Rust Style

- Prefer functional/iterator style over imperative loops
- Avoid mutable variables where possible; favor expression-oriented code
- Minimize side effects; keep functions pure
- Use combinator chains over `for` loops with `mut` accumulators

## Architecture

- `src/main.rs` — CLI entrypoint, command registration, panic hook
- `src/error.rs` — AppError enum -> CommandError mapping
- `src/neo4j_client.rs` — Graph connection from ExecutionContext + env vars
- `src/convert.rs` — Neo4j Row/Node/Relation -> serde_json::Value
- `src/commands/` — One file per command group (ping, query, schema, nodes, rels)

## Connection

Resolution order: CLI flags > env vars > defaults.
Env vars: `NEO4J_URI`, `NEO4J_USER`, `NEO4J_PASSWORD`, `NEO4J_DB`.
Defaults: bolt://localhost:7687, user=neo4j, db=neo4j. Password required.

## Testing

```bash
docker run -p 7687:7687 -e NEO4J_AUTH=neo4j/testpass neo4j:5
NEO4J_PASSWORD=testpass cargo run -- ping
```
