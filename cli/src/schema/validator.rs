use anyhow::{Context, Result};
use jsonschema::JSONSchema;
use serde_json::Value;

/// Schema validator for PlanSpec resources
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
    pub fn validate(&self, resource: &Value) -> Result<(), Vec<String>> {
        let kind = resource
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("");

        let schema = match kind {
            "Goal" => &self.goal_schema,
            "Plan" => &self.plan_schema,
            "Capability" => &self.capability_schema,
            "Binding" => &self.binding_schema,
            "Execution" => &self.execution_schema,
            _ => return Err(vec![format!("Unknown resource kind: {}", kind)]),
        };

        let result = schema.validate(resource);

        if let Err(errors) = result {
            let error_messages: Vec<String> = errors
                .map(|e| {
                    let path = e.instance_path.to_string();
                    let path_str = if path.is_empty() { "(root)".to_string() } else { path };
                    format!("{}: {}", path_str, e)
                })
                .collect();

            if error_messages.is_empty() {
                Ok(())
            } else {
                Err(error_messages)
            }
        } else {
            Ok(())
        }
    }
}

fn compile_schema(schema_json: &str) -> Result<JSONSchema> {
    let schema: Value = serde_json::from_str(schema_json)
        .context("Failed to parse schema JSON")?;

    JSONSchema::compile(&schema)
        .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))
}

// Embedded schemas (simplified for offline validation)
// These are reduced versions focusing on required fields and basic structure

const GOAL_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": {
      "type": "string",
      "const": "planspec.io/v1alpha1"
    },
    "kind": {
      "type": "string",
      "const": "Goal"
    },
    "metadata": {
      "type": "object",
      "required": ["name", "namespace"],
      "properties": {
        "name": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        },
        "namespace": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        },
        "labels": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        },
        "annotations": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        }
      }
    },
    "spec": {
      "type": "object",
      "required": ["description"],
      "properties": {
        "description": {
          "type": "string",
          "minLength": 1
        },
        "constraints": {
          "type": "object"
        },
        "acceptanceCriteria": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["description"],
            "properties": {
              "description": { "type": "string" }
            }
          }
        },
        "planSelector": {
          "type": "object"
        },
        "timeout": {
          "type": "string"
        },
        "priority": {
          "type": "integer",
          "minimum": 0
        },
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
    "status": {
      "type": "object"
    }
  }
}
"#;

const PLAN_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": {
      "type": "string",
      "const": "planspec.io/v1alpha1"
    },
    "kind": {
      "type": "string",
      "const": "Plan"
    },
    "metadata": {
      "type": "object",
      "required": ["name", "namespace"],
      "properties": {
        "name": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        },
        "namespace": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        },
        "labels": {
          "type": "object",
          "additionalProperties": { "type": "string" }
        }
      }
    },
    "spec": {
      "type": "object",
      "required": ["description", "graph"],
      "properties": {
        "description": {
          "type": "string",
          "minLength": 1
        },
        "goalRef": {
          "type": "object",
          "properties": {
            "name": { "type": "string" },
            "namespace": { "type": "string" }
          }
        },
        "series": { "type": "string" },
        "version": { "type": "string" },
        "supersedes": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "name": { "type": "string" }
            }
          }
        },
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
                  "id": {
                    "type": "string",
                    "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
                  },
                  "kind": {
                    "type": "string",
                    "enum": ["Task", "Gate", "Group", "External"]
                  },
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
                  "type": {
                    "type": "string",
                    "enum": ["hard", "soft"]
                  }
                }
              }
            }
          }
        }
      }
    },
    "status": {
      "type": "object"
    }
  }
}
"#;

const CAPABILITY_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": {
      "type": "string",
      "const": "planspec.io/v1alpha1"
    },
    "kind": {
      "type": "string",
      "const": "Capability"
    },
    "metadata": {
      "type": "object",
      "required": ["name", "namespace"],
      "properties": {
        "name": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        },
        "namespace": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        }
      }
    },
    "spec": {
      "type": "object",
      "required": ["description"],
      "properties": {
        "displayName": { "type": "string" },
        "description": {
          "type": "string",
          "minLength": 1
        },
        "category": { "type": "string" },
        "inputs": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["name", "type"],
            "properties": {
              "name": { "type": "string" },
              "type": {
                "type": "string",
                "enum": ["string", "number", "boolean", "object", "array"]
              },
              "description": { "type": "string" },
              "required": { "type": "boolean" },
              "default": {}
            }
          }
        },
        "outputs": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["name", "type"],
            "properties": {
              "name": { "type": "string" },
              "type": { "type": "string" }
            }
          }
        },
        "requirements": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["capability"],
            "properties": {
              "capability": { "type": "string" },
              "optional": { "type": "boolean" }
            }
          }
        }
      }
    },
    "status": {
      "type": "object"
    }
  }
}
"#;

const BINDING_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": {
      "type": "string",
      "const": "planspec.io/v1alpha1"
    },
    "kind": {
      "type": "string",
      "const": "Binding"
    },
    "metadata": {
      "type": "object",
      "required": ["name", "namespace"],
      "properties": {
        "name": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        },
        "namespace": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        }
      }
    },
    "spec": {
      "type": "object",
      "required": ["rules"],
      "properties": {
        "rules": {
          "type": "array",
          "minItems": 1,
          "items": {
            "type": "object",
            "required": ["selector", "target"],
            "properties": {
              "selector": {
                "type": "object",
                "properties": {
                  "capabilityRef": { "type": "object" },
                  "planRef": { "type": "object" },
                  "nodeId": { "type": "string" }
                }
              },
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
    "status": {
      "type": "object"
    }
  }
}
"#;

const EXECUTION_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["apiVersion", "kind", "metadata", "spec"],
  "properties": {
    "apiVersion": {
      "type": "string",
      "const": "planspec.io/v1alpha1"
    },
    "kind": {
      "type": "string",
      "const": "Execution"
    },
    "metadata": {
      "type": "object",
      "required": ["name", "namespace"],
      "properties": {
        "name": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        },
        "namespace": {
          "type": "string",
          "pattern": "^[a-z0-9]([-a-z0-9]*[a-z0-9])?$"
        },
        "ownerReferences": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["apiVersion", "kind", "name", "uid"],
            "properties": {
              "apiVersion": { "type": "string" },
              "kind": { "type": "string" },
              "name": { "type": "string" },
              "uid": { "type": "string" }
            }
          }
        }
      }
    },
    "spec": {
      "type": "object",
      "required": ["planRef"],
      "properties": {
        "goalRef": {
          "type": "object",
          "properties": {
            "name": { "type": "string" },
            "namespace": { "type": "string" }
          }
        },
        "planRef": {
          "type": "object",
          "required": ["name"],
          "properties": {
            "name": { "type": "string" },
            "namespace": { "type": "string" }
          }
        },
        "bindingRef": {
          "type": "object",
          "properties": {
            "name": { "type": "string" },
            "namespace": { "type": "string" }
          }
        },
        "runtimeRef": {
          "type": "object"
        },
        "parameters": {
          "type": "object"
        },
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
    "status": {
      "type": "object"
    }
  }
}
"#;
