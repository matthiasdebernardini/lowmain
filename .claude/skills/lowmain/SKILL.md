---
name: lowmain
description: Agent-native Neo4j CLI — explore schemas, query graphs, and manage nodes and relationships
---

# lowmain — Neo4j CLI for Agents

lowmain is an agent-optimized Neo4j command-line tool. All output is structured JSON with next-action suggestions, error codes, and fix hints. This skill enables you to interact with Neo4j databases directly from the terminal.

## When to Use

Invoke `/lowmain` when you need to:

- Connect to and explore a Neo4j database
- Run Cypher queries (read or write)
- Inspect database schema (labels, relationship types, indexes, constraints)
- Create, read, update, or delete nodes
- Create, find, or delete relationships
- Debug Neo4j connectivity or authentication issues

## Prerequisites

1. **lowmain binary** — built and available in PATH (or run via `cargo run --` from the repo)
2. **Neo4j instance** — running and accessible (default: `bolt://localhost:7687`)
3. **NEO4J_PASSWORD** — set as an environment variable or passed via `--password`

Quick setup for local testing:

```bash
docker run -p 7687:7687 -e NEO4J_AUTH=neo4j/testpass neo4j:5
export NEO4J_PASSWORD=testpass
```

## Instructions

Follow this phased workflow when working with a Neo4j database.

### Phase 1: Connect

Verify connectivity before doing anything else.

```bash
lowmain ping
```

If this fails, check:
- Neo4j is running and reachable at the configured URI
- `NEO4J_PASSWORD` is set (or pass `--password`)
- The URI scheme is correct (`bolt://` for Bolt protocol)

### Phase 2: Orient

Understand the database structure before querying or mutating.

```bash
lowmain schema              # full overview: labels, types, indexes, constraints
lowmain schema labels       # just node labels
lowmain schema types        # just relationship types
lowmain schema count        # node and relationship totals
lowmain schema indexes      # list indexes
lowmain schema constraints  # list constraints
```

Use schema output to determine what labels and relationship types exist before writing queries.

### Phase 3: Operate

Now query, create, update, or delete data.

**Read data** with `query` or `node find`:

```bash
lowmain query "MATCH (n:Person) RETURN n" --limit=10
lowmain node find --label=Person --where="name=Alice"
```

**Write data** with `node create`, `node update`, `rel create`, or `query --write`:

```bash
lowmain node create --label=Person --props='{"name":"Alice","age":30}'
lowmain node update 42 --set='{"age":31}'
lowmain rel create --from=42 --to=99 --type=KNOWS --props='{"since":2024}'
```

**Delete data** with `node delete` or `rel delete`:

```bash
lowmain node delete 42 --detach
lowmain rel delete 7
```

## Command Reference

### Global Connection Flags

Available on every command. Resolution order: CLI flag > environment variable > default.

| Flag | Env Var | Default | Description |
|------|---------|---------|-------------|
| `--uri` | `NEO4J_URI` | `bolt://localhost:7687` | Neo4j connection URI |
| `--user` | `NEO4J_USER` | `neo4j` | Authentication username |
| `--password` | `NEO4J_PASSWORD` | *(required)* | Authentication password |
| `--db` | `NEO4J_DB` | `neo4j` | Database name |

### `lowmain ping`

Test Neo4j connection health.

```bash
lowmain ping
```

Returns: `{ connected, uri, db }`

### `lowmain query <cypher>`

Execute a raw Cypher query.

| Argument/Flag | Required | Default | Description |
|---------------|----------|---------|-------------|
| `<cypher>` | Yes | — | Cypher query string (positional) |
| `--params` | No | — | JSON object of query parameters |
| `--limit` | No | 100 | Max rows to return |
| `--write` | No | false | Execute as a write operation (no rows returned) |

```bash
# Read query with parameters
lowmain query "MATCH (n:Person) WHERE n.name = \$name RETURN n" --params='{"name":"Alice"}'

# Write query
lowmain query "CREATE (n:Log {msg: \$msg})" --params='{"msg":"hello"}' --write

# Limit results
lowmain query "MATCH (n) RETURN n" --limit=5
```

### `lowmain schema [subcommand]`

Introspect database structure. With no subcommand, returns everything.

| Subcommand | Description | Returns |
|------------|-------------|---------|
| *(none)* | Full schema overview | labels, relationship_types, indexes, constraints |
| `labels` | Node labels | `{ labels: [] }` |
| `types` | Relationship types | `{ relationship_types: [] }` |
| `indexes` | Database indexes | `{ indexes: [] }` |
| `constraints` | Database constraints | `{ constraints: [] }` |
| `count` | Node and relationship counts | `{ node_count, relationship_count }` |

### `lowmain node find`

Find nodes by label with optional filtering.

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `--label` | Yes | — | Node label to search |
| `--where` | No | — | Filter as `property=value` |
| `--limit` | No | 100 | Max nodes to return |

```bash
lowmain node find --label=Person
lowmain node find --label=Person --where="name=Alice" --limit=50
```

### `lowmain node get <id>`

Get a single node by internal ID.

```bash
lowmain node get 42
```

Returns: `{ node: { _id, _labels, ...properties } }`

### `lowmain node create`

Create a new node.

| Flag | Required | Description |
|------|----------|-------------|
| `--label` | Yes | Node label |
| `--props` | Yes | JSON object of properties |

```bash
lowmain node create --label=Person --props='{"name":"Bob","age":25}'
```

### `lowmain node update <id>`

Update properties on an existing node.

| Argument/Flag | Required | Description |
|---------------|----------|-------------|
| `<id>` | Yes | Node ID (positional) |
| `--set` | Yes | JSON object of properties to set |

```bash
lowmain node update 42 --set='{"age":31,"status":"active"}'
```

### `lowmain node delete <id>`

Delete a node by ID.

| Argument/Flag | Required | Description |
|---------------|----------|-------------|
| `<id>` | Yes | Node ID (positional) |
| `--detach` | No | Use DETACH DELETE to also remove relationships |

```bash
lowmain node delete 42
lowmain node delete 42 --detach
```

### `lowmain rel find`

Find relationships by type and/or endpoints. All flags are optional.

| Flag | Default | Description |
|------|---------|-------------|
| `--from` | — | Source node ID |
| `--to` | — | Target node ID |
| `--type` | — | Relationship type |
| `--limit` | 100 | Max results |

```bash
lowmain rel find --type=KNOWS
lowmain rel find --from=42 --to=99
lowmain rel find --type=KNOWS --from=42 --limit=20
```

### `lowmain rel create`

Create a relationship between two nodes.

| Flag | Required | Description |
|------|----------|-------------|
| `--from` | Yes | Source node ID |
| `--to` | Yes | Target node ID |
| `--type` | Yes | Relationship type |
| `--props` | No | JSON object of relationship properties |

```bash
lowmain rel create --from=42 --to=99 --type=KNOWS
lowmain rel create --from=42 --to=99 --type=WORKS_AT --props='{"since":2024}'
```

### `lowmain rel delete <id>`

Delete a relationship by ID.

```bash
lowmain rel delete 7
```

## Best Practices

1. **Always use parameterized queries.** Pass values via `--params` instead of string interpolation to prevent Cypher injection.

   ```bash
   # Good
   lowmain query "MATCH (n) WHERE n.name = \$name RETURN n" --params='{"name":"Alice"}'

   # Bad — injection risk
   lowmain query "MATCH (n) WHERE n.name = 'Alice' RETURN n"
   ```

2. **Orient before operating.** Run `lowmain schema` first to discover labels and relationship types. Don't guess at names.

3. **Follow next-action suggestions.** Every response includes suggested next commands. Use them to navigate the graph.

4. **Use `--detach` carefully.** `node delete --detach` removes the node and all its relationships. Without `--detach`, deletion fails if the node has relationships.

5. **Handle errors by code.** Errors include a `code` field (`CONNECTION_FAILED`, `NODE_NOT_FOUND`, `CYPHER_SYNTAX_ERROR`, etc.) and a `fix` field with remediation steps. Use the code for programmatic handling and the fix for recovery.

6. **Prefer structured commands over raw Cypher.** Use `node find`, `node create`, `rel create` etc. when possible. Fall back to `query` for complex traversals or aggregations.

7. **Set `NEO4J_PASSWORD` in your environment** rather than passing `--password` on every call.

## Error Codes

| Code | Retryable | Meaning |
|------|-----------|---------|
| `CONNECTION_FAILED` | Yes | Cannot reach Neo4j — check URI and that the server is running |
| `AUTH_FAILED` | No | Bad username or password |
| `CONNECTION_NOT_CONFIGURED` | No | `NEO4J_PASSWORD` not set and `--password` not passed |
| `CYPHER_SYNTAX_ERROR` | No | Invalid Cypher — check syntax |
| `CONSTRAINT_VIOLATION` | No | Unique constraint violated |
| `QUERY_FAILED` | No | General query error |
| `NODE_NOT_FOUND` | No | No node with that ID |
| `REL_NOT_FOUND` | No | No relationship with that ID |
| `INVALID_PARAMS` | No | `--params` is not valid JSON |

## Example Interaction

A full session exploring a database and creating data:

```bash
# 1. Connect
lowmain ping
# => { "connected": true, "uri": "bolt://localhost:7687", "db": "neo4j" }

# 2. Orient — check what's in the database
lowmain schema
# => { "labels": ["Person","Company"], "relationship_types": ["WORKS_AT","KNOWS"], ... }

lowmain schema count
# => { "node_count": 150, "relationship_count": 320 }

# 3. Explore existing data
lowmain node find --label=Person --limit=5
# => { "nodes": [{ "_id": 0, "_labels": ["Person"], "name": "Alice", "age": 30 }, ...], "count": 5 }

lowmain rel find --type=WORKS_AT --from=0
# => { "relationships": [{ "_id": 10, "_type": "WORKS_AT", "_start_node_id": 0, "_end_node_id": 50, "since": 2022 }] }

# 4. Create new data
lowmain node create --label=Person --props='{"name":"Charlie","age":28}'
# => { "created": true, "node": { "_id": 151, "_labels": ["Person"], "name": "Charlie", "age": 28 } }

lowmain rel create --from=151 --to=50 --type=WORKS_AT --props='{"since":2025}'
# => { "created": true, "relationship": { "_id": 321, "_type": "WORKS_AT", ... } }

# 5. Update
lowmain node update 151 --set='{"title":"Engineer"}'
# => { "updated": true, "node": { "_id": 151, "name": "Charlie", "age": 28, "title": "Engineer" } }

# 6. Complex query
lowmain query "MATCH (p:Person)-[:WORKS_AT]->(c:Company) RETURN p.name, c.name" --limit=10
# => { "rows": [{"p.name": "Alice", "c.name": "Acme"}, ...], "count": 10 }
```
