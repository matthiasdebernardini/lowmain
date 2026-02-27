#[allow(unused_imports)]
use neo4rs::{Node, Path, Relation, Row, UnboundedRelation};
use serde_json::{Map, Value, json};

/// Convert a Neo4j Row to a JSON Value using serde deserialization.
pub fn row_to_json(row: &Row) -> Value {
    row.to::<Value>().unwrap_or(Value::Null)
}

/// Convert a Neo4j Node to a JSON Value.
pub fn node_to_json(node: &Node) -> Value {
    let mut map = Map::new();
    map.insert("_id".to_string(), json!(node.id()));
    map.insert("_labels".to_string(), json!(node.labels()));

    for key in node.keys() {
        let val = node_field_to_json(node, key);
        map.insert(key.to_string(), val);
    }

    Value::Object(map)
}

/// Extract a single property from a Node.
fn node_field_to_json(node: &Node, key: &str) -> Value {
    if let Ok(v) = node.get::<i64>(key) {
        return json!(v);
    }
    if let Ok(v) = node.get::<f64>(key) {
        return json!(v);
    }
    if let Ok(v) = node.get::<bool>(key) {
        return json!(v);
    }
    if let Ok(v) = node.get::<String>(key) {
        return json!(v);
    }
    if let Ok(v) = node.get::<Vec<String>>(key) {
        return json!(v);
    }
    if let Ok(v) = node.get::<Vec<i64>>(key) {
        return json!(v);
    }
    Value::Null
}

/// Convert a Neo4j Relation to a JSON Value.
pub fn relation_to_json(rel: &Relation) -> Value {
    let mut map = Map::new();
    map.insert("_id".to_string(), json!(rel.id()));
    map.insert("_start_node_id".to_string(), json!(rel.start_node_id()));
    map.insert("_end_node_id".to_string(), json!(rel.end_node_id()));
    map.insert("_type".to_string(), json!(rel.typ()));

    for key in rel.keys() {
        let val = rel_field_to_json(rel, key);
        map.insert(key.to_string(), val);
    }

    Value::Object(map)
}

/// Extract a single property from a Relation.
fn rel_field_to_json(rel: &Relation, key: &str) -> Value {
    if let Ok(v) = rel.get::<i64>(key) {
        return json!(v);
    }
    if let Ok(v) = rel.get::<f64>(key) {
        return json!(v);
    }
    if let Ok(v) = rel.get::<bool>(key) {
        return json!(v);
    }
    if let Ok(v) = rel.get::<String>(key) {
        return json!(v);
    }
    Value::Null
}

/// Convert an UnboundedRelation to a JSON Value.
#[allow(dead_code)]
fn unbounded_rel_to_json(rel: &UnboundedRelation) -> Value {
    let mut map = Map::new();
    map.insert("_id".to_string(), json!(rel.id()));
    map.insert("_type".to_string(), json!(rel.typ()));

    for key in rel.keys() {
        let val = if let Ok(v) = rel.get::<String>(key) {
            json!(v)
        } else if let Ok(v) = rel.get::<i64>(key) {
            json!(v)
        } else if let Ok(v) = rel.get::<f64>(key) {
            json!(v)
        } else if let Ok(v) = rel.get::<bool>(key) {
            json!(v)
        } else {
            Value::Null
        };
        map.insert(key.to_string(), val);
    }

    Value::Object(map)
}

/// Convert a Neo4j Path to a JSON Value.
#[allow(dead_code)]
fn path_to_json(path: &Path) -> Value {
    let nodes: Vec<Value> = path.nodes().iter().map(node_to_json).collect();
    let rels: Vec<Value> = path.rels().iter().map(unbounded_rel_to_json).collect();
    json!({
        "_type": "path",
        "nodes": nodes,
        "relationships": rels,
    })
}
