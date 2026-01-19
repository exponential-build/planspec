use anyhow::{Context, Result};
use jsonschema::JSONSchema;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Validates PlanSpec resources
pub struct Validator {
    goal_schema: JSONSchema,
    plan_schema: JSONSchema,
    capability_schema: JSONSchema,
    binding_schema: JSONSchema,
    execution_schema: JSONSchema,
}

impl Validator {
    /// Create a new validator with embedded schemas
    pub fn new() -> Result<Self> {
        Ok(Self {
            goal_schema: compile_schema(GOAL_SCHEMA)?,
            plan_schema: compile_schema(PLAN_SCHEMA)?,
            capability_schema: compile_schema(CAPABILITY_SCHEMA)?,
            binding_schema: compile_schema(BINDING_SCHEMA)?,
            execution_schema: compile_schema(EXECUTION_SCHEMA)?,
        })
    }

    /// Validate a resource against its schema
    pub fn validate(&self, resource: &Value) -> Result<Vec<String>> {
        let kind = resource
            .get("kind")
            .and_then(|k| k.as_str())
            .ok_or_else(|| anyhow::anyhow!("Resource missing 'kind' field"))?;

        let schema = match kind {
            "Goal" => &self.goal_schema,
            "Plan" => &self.plan_schema,
            "Capability" => &self.capability_schema,
            "Binding" => &self.binding_schema,
            "Execution" => &self.execution_schema,
            _ => return Err(anyhow::anyhow!("Unknown resource kind: {}", kind)),
        };

        let result = schema.validate(resource);

        match result {
            Ok(_) => {
                // Additional validation for Plans: check DAG acyclicity
                if kind == "Plan" {
                    self.validate_plan_dag(resource)?;
                }
                Ok(vec![])
            }
            Err(errors) => {
                let messages: Vec<String> = errors
                    .map(|e| format!("{}: {}", e.instance_path, e))
                    .collect();
                Ok(messages)
            }
        }
    }

    /// Validate that a Plan's graph is acyclic
    fn validate_plan_dag(&self, plan: &Value) -> Result<()> {
        let graph = plan
            .get("spec")
            .and_then(|s| s.get("graph"))
            .ok_or_else(|| anyhow::anyhow!("Plan missing spec.graph"))?;

        let nodes = graph
            .get("nodes")
            .and_then(|n| n.as_array())
            .ok_or_else(|| anyhow::anyhow!("Plan graph missing nodes array"))?;

        let empty_edges = vec![];
        let edges = graph
            .get("edges")
            .and_then(|e| e.as_array())
            .unwrap_or(&empty_edges);

        // Build node set
        let node_ids: HashSet<&str> = nodes
            .iter()
            .filter_map(|n| n.get("id").and_then(|i| i.as_str()))
            .collect();

        // Validate edge references and build adjacency list
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        for edge in edges {
            let from = edge
                .get("from")
                .and_then(|f| f.as_str())
                .ok_or_else(|| anyhow::anyhow!("Edge missing 'from' field"))?;

            let to = edge
                .get("to")
                .and_then(|t| t.as_str())
                .ok_or_else(|| anyhow::anyhow!("Edge missing 'to' field"))?;

            if !node_ids.contains(from) {
                anyhow::bail!("Edge references unknown node: {}", from);
            }
            if !node_ids.contains(to) {
                anyhow::bail!("Edge references unknown node: {}", to);
            }

            adjacency.entry(from).or_default().push(to);
        }

        // Check for cycles using DFS
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();

        for node in &node_ids {
            if !visited.contains(node) {
                if self.has_cycle(node, &adjacency, &mut visited, &mut in_stack) {
                    anyhow::bail!("Plan graph contains a cycle");
                }
            }
        }

        Ok(())
    }

    fn has_cycle<'a>(
        &self,
        node: &'a str,
        adjacency: &HashMap<&'a str, Vec<&'a str>>,
        visited: &mut HashSet<&'a str>,
        in_stack: &mut HashSet<&'a str>,
    ) -> bool {
        visited.insert(node);
        in_stack.insert(node);

        if let Some(neighbors) = adjacency.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle(neighbor, adjacency, visited, in_stack) {
                        return true;
                    }
                } else if in_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        in_stack.remove(node);
        false
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new().expect("Failed to create validator")
    }
}

fn compile_schema(schema_json: &str) -> Result<JSONSchema> {
    let schema: Value = serde_json::from_str(schema_json)
        .context("Failed to parse schema JSON")?;

    JSONSchema::compile(&schema)
        .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))
}

// Embedded schemas (self-contained, no external $ref)

const GOAL_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": { "type": "string", "const": "planspec.io/v1alpha1" },
    "kind": { "type": "string", "const": "Goal" },
    "metadata": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string", "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$" },
        "namespace": { "type": "string" },
        "labels": { "type": "object", "additionalProperties": { "type": "string" } },
        "annotations": { "type": "object", "additionalProperties": { "type": "string" } }
      }
    },
    "spec": {
      "type": "object",
      "required": ["description"],
      "properties": {
        "description": { "type": "string", "minLength": 1 },
        "constraints": { "type": "object" },
        "acceptanceCriteria": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["description"],
            "properties": { "description": { "type": "string" } }
          }
        },
        "planSelector": { "type": "object" },
        "timeout": { "type": "string" },
        "priority": { "type": "integer", "minimum": 0 },
        "context": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["format"],
            "properties": {
              "name": { "type": "string" },
              "format": { "type": "string", "enum": ["markdown", "text", "json", "uri-list"] },
              "content": {},
              "uris": { "type": "array", "items": { "type": "string" } }
            }
          }
        }
      }
    },
    "status": { "type": "object" }
  }
}
"#;

const PLAN_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": { "type": "string", "const": "planspec.io/v1alpha1" },
    "kind": { "type": "string", "const": "Plan" },
    "metadata": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string", "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$" },
        "namespace": { "type": "string" },
        "labels": { "type": "object", "additionalProperties": { "type": "string" } }
      }
    },
    "spec": {
      "type": "object",
      "required": ["description", "graph"],
      "properties": {
        "description": { "type": "string", "minLength": 1 },
        "goalRef": { "type": "object" },
        "series": { "type": "string" },
        "version": { "type": "string" },
        "supersedes": { "type": "array", "items": { "type": "object" } },
        "context": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["format"],
            "properties": {
              "name": { "type": "string" },
              "format": { "type": "string", "enum": ["markdown", "text", "json", "uri-list"] },
              "content": {},
              "uris": { "type": "array", "items": { "type": "string" } }
            }
          }
        },
        "graph": {
          "type": "object",
          "required": ["nodes"],
          "properties": {
            "nodes": {
              "type": "array",
              "minItems": 1,
              "items": {
                "type": "object",
                "required": ["id", "kind", "description"],
                "properties": {
                  "id": { "type": "string" },
                  "kind": { "type": "string", "enum": ["Task", "Gate", "Group", "External"] },
                  "description": { "type": "string" },
                  "capabilityRef": { "type": "object" },
                  "inputs": { "type": "object" },
                  "outputs": { "type": "array" },
                  "timeout": { "type": "string" },
                  "retries": { "type": "integer" },
                  "when": { "type": "string" },
                  "context": {
                    "type": "array",
                    "items": {
                      "type": "object",
                      "required": ["format"],
                      "properties": {
                        "name": { "type": "string" },
                        "format": { "type": "string", "enum": ["markdown", "text", "json", "uri-list"] },
                        "content": {},
                        "uris": { "type": "array", "items": { "type": "string" } }
                      }
                    }
                  }
                }
              }
            },
            "edges": {
              "type": "array",
              "items": {
                "type": "object",
                "required": ["from", "to"],
                "properties": {
                  "from": { "type": "string" },
                  "to": { "type": "string" },
                  "type": { "type": "string", "enum": ["hard", "soft"] }
                }
              }
            }
          }
        }
      }
    },
    "status": { "type": "object" }
  }
}
"#;

const CAPABILITY_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": { "type": "string", "const": "planspec.io/v1alpha1" },
    "kind": { "type": "string", "const": "Capability" },
    "metadata": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string" },
        "namespace": { "type": "string" }
      }
    },
    "spec": {
      "type": "object",
      "required": ["description"],
      "properties": {
        "displayName": { "type": "string" },
        "description": { "type": "string", "minLength": 1 },
        "category": { "type": "string" },
        "inputs": { "type": "array" },
        "outputs": { "type": "array" },
        "requirements": { "type": "array" }
      }
    },
    "status": { "type": "object" }
  }
}
"#;

const BINDING_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": { "type": "string", "const": "planspec.io/v1alpha1" },
    "kind": { "type": "string", "const": "Binding" },
    "metadata": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string" },
        "namespace": { "type": "string" }
      }
    },
    "spec": {
      "type": "object",
      "properties": {
        "rules": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["selector", "target"],
            "properties": {
              "selector": { "type": "object" },
              "target": {
                "type": "object",
                "required": ["provider"],
                "properties": {
                  "provider": { "type": "string" },
                  "config": { "type": "object" }
                }
              }
            }
          }
        }
      }
    },
    "status": { "type": "object" }
  }
}
"#;

const EXECUTION_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": { "type": "string", "const": "planspec.io/v1alpha1" },
    "kind": { "type": "string", "const": "Execution" },
    "metadata": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "type": "string" },
        "namespace": { "type": "string" },
        "ownerReferences": { "type": "array" }
      }
    },
    "spec": {
      "type": "object",
      "required": ["planRef"],
      "properties": {
        "goalRef": { "type": "object" },
        "planRef": {
          "type": "object",
          "required": ["name"],
          "properties": {
            "name": { "type": "string" },
            "namespace": { "type": "string" }
          }
        },
        "bindingRef": { "type": "object" },
        "runtimeRef": { "type": "object" },
        "parameters": { "type": "object" },
        "context": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["format"],
            "properties": {
              "name": { "type": "string" },
              "format": { "type": "string", "enum": ["markdown", "text", "json", "uri-list"] },
              "content": {},
              "uris": { "type": "array", "items": { "type": "string" } }
            }
          }
        }
      }
    },
    "status": { "type": "object" }
  }
}
"#;
