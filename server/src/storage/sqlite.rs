use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::Value;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use uuid::Uuid;

use planspec_core::{WatchEvent, WatchEventType};

use super::types::StoredObject;

/// Type alias for the database row tuple to reduce complexity warnings
type DbRow = (String, String, String, String, String, i64, i64, String, String);

/// SQLite-backed storage for PlanSpec resources
#[derive(Clone)]
pub struct Store {
    pool: Pool<Sqlite>,
}

impl Store {
    /// Create a new store, initializing the database schema
    pub async fn new(path: &str) -> Result<Self> {
        let url = format!("sqlite:{}?mode=rwc", path);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .context("Failed to connect to SQLite database")?;

        // Initialize schema
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS resources (
                namespace TEXT NOT NULL,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                object_json TEXT NOT NULL,
                uid TEXT NOT NULL,
                resource_version INTEGER NOT NULL,
                generation INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (namespace, kind, name)
            );

            CREATE INDEX IF NOT EXISTS idx_resources_kind ON resources(kind);
            CREATE INDEX IF NOT EXISTS idx_resources_ns_kind ON resources(namespace, kind);
            "#,
        )
        .execute(&pool)
        .await
        .context("Failed to initialize database schema")?;

        // Create revision counter table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS revision (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                current INTEGER NOT NULL DEFAULT 0
            );
            INSERT OR IGNORE INTO revision (id, current) VALUES (1, 0);
            "#,
        )
        .execute(&pool)
        .await
        .context("Failed to initialize revision counter")?;

        Ok(Self { pool })
    }

    /// Get the next revision number (atomically increments)
    async fn next_revision(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "UPDATE revision SET current = current + 1 WHERE id = 1 RETURNING current",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// Create a new resource
    pub async fn create(
        &self,
        namespace: &str,
        kind: &str,
        name: &str,
        mut object: Value,
    ) -> Result<(StoredObject, WatchEvent)> {
        let now = Utc::now();
        let uid = Uuid::new_v4().to_string();
        let resource_version = self.next_revision().await?;
        let generation = 1i64;

        // Inject server-managed metadata
        if let Some(metadata) = object.get_mut("metadata").and_then(|m| m.as_object_mut()) {
            metadata.insert("uid".to_string(), Value::String(uid.clone()));
            metadata.insert(
                "resourceVersion".to_string(),
                Value::String(resource_version.to_string()),
            );
            metadata.insert("generation".to_string(), Value::Number(generation.into()));
            metadata.insert(
                "creationTimestamp".to_string(),
                Value::String(now.to_rfc3339()),
            );
        }

        let object_json = serde_json::to_string(&object)?;

        sqlx::query(
            r#"
            INSERT INTO resources (namespace, kind, name, object_json, uid, resource_version, generation, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(namespace)
        .bind(kind)
        .bind(name)
        .bind(&object_json)
        .bind(&uid)
        .bind(resource_version)
        .bind(generation)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .context("Failed to insert resource")?;

        let stored = StoredObject {
            namespace: namespace.to_string(),
            kind: kind.to_string(),
            name: name.to_string(),
            object: object.clone(),
            uid,
            resource_version,
            generation,
            created_at: now,
            updated_at: now,
        };

        let event = WatchEvent {
            event_type: WatchEventType::Added,
            object,
            rev: resource_version.to_string(),
        };

        Ok((stored, event))
    }

    /// Get a resource by key
    pub async fn get(
        &self,
        namespace: &str,
        kind: &str,
        name: &str,
    ) -> Result<Option<StoredObject>> {
        let row: Option<DbRow> =
            sqlx::query_as(
                r#"
                SELECT namespace, kind, name, object_json, uid, resource_version, generation, created_at, updated_at
                FROM resources
                WHERE namespace = ? AND kind = ? AND name = ?
                "#,
            )
            .bind(namespace)
            .bind(kind)
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some((ns, k, n, obj_json, uid, rv, gen, created, updated)) => {
                let object: Value = serde_json::from_str(&obj_json)?;
                Ok(Some(StoredObject {
                    namespace: ns,
                    kind: k,
                    name: n,
                    object,
                    uid,
                    resource_version: rv,
                    generation: gen,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created)?.with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated)?.with_timezone(&Utc),
                }))
            }
            None => Ok(None),
        }
    }

    /// List resources by namespace and kind
    pub async fn list(&self, namespace: &str, kind: &str) -> Result<Vec<StoredObject>> {
        let rows: Vec<DbRow> =
            sqlx::query_as(
                r#"
                SELECT namespace, kind, name, object_json, uid, resource_version, generation, created_at, updated_at
                FROM resources
                WHERE namespace = ? AND kind = ?
                ORDER BY name
                "#,
            )
            .bind(namespace)
            .bind(kind)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|(ns, k, n, obj_json, uid, rv, gen, created, updated)| {
                let object: Value = serde_json::from_str(&obj_json)?;
                Ok(StoredObject {
                    namespace: ns,
                    kind: k,
                    name: n,
                    object,
                    uid,
                    resource_version: rv,
                    generation: gen,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created)?.with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated)?.with_timezone(&Utc),
                })
            })
            .collect()
    }

    /// List all resources of a kind across all namespaces
    pub async fn list_all(&self, kind: &str) -> Result<Vec<StoredObject>> {
        let rows: Vec<DbRow> =
            sqlx::query_as(
                r#"
                SELECT namespace, kind, name, object_json, uid, resource_version, generation, created_at, updated_at
                FROM resources
                WHERE kind = ?
                ORDER BY namespace, name
                "#,
            )
            .bind(kind)
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter()
            .map(|(ns, k, n, obj_json, uid, rv, gen, created, updated)| {
                let object: Value = serde_json::from_str(&obj_json)?;
                Ok(StoredObject {
                    namespace: ns,
                    kind: k,
                    name: n,
                    object,
                    uid,
                    resource_version: rv,
                    generation: gen,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created)?.with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated)?.with_timezone(&Utc),
                })
            })
            .collect()
    }

    /// Replace a resource (with optimistic concurrency check)
    pub async fn replace(
        &self,
        namespace: &str,
        kind: &str,
        name: &str,
        mut object: Value,
        expected_resource_version: Option<&str>,
    ) -> Result<(StoredObject, WatchEvent)> {
        // Get existing resource
        let existing = self
            .get(namespace, kind, name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Resource not found"))?;

        // Check resourceVersion if provided
        if let Some(expected_rv) = expected_resource_version {
            let expected: i64 = expected_rv.parse()?;
            if existing.resource_version != expected {
                anyhow::bail!(
                    "Conflict: resourceVersion mismatch (expected {}, got {})",
                    expected_rv,
                    existing.resource_version
                );
            }
        }

        let now = Utc::now();
        let resource_version = self.next_revision().await?;

        // Check if spec changed to determine generation increment
        let old_spec = existing.object.get("spec");
        let new_spec = object.get("spec");
        let generation = if old_spec != new_spec {
            existing.generation + 1
        } else {
            existing.generation
        };

        // Preserve immutable metadata, update mutable parts
        if let Some(metadata) = object.get_mut("metadata").and_then(|m| m.as_object_mut()) {
            metadata.insert("uid".to_string(), Value::String(existing.uid.clone()));
            metadata.insert(
                "resourceVersion".to_string(),
                Value::String(resource_version.to_string()),
            );
            metadata.insert("generation".to_string(), Value::Number(generation.into()));
            metadata.insert(
                "creationTimestamp".to_string(),
                Value::String(existing.created_at.to_rfc3339()),
            );
        }

        let object_json = serde_json::to_string(&object)?;

        sqlx::query(
            r#"
            UPDATE resources
            SET object_json = ?, resource_version = ?, generation = ?, updated_at = ?
            WHERE namespace = ? AND kind = ? AND name = ?
            "#,
        )
        .bind(&object_json)
        .bind(resource_version)
        .bind(generation)
        .bind(now.to_rfc3339())
        .bind(namespace)
        .bind(kind)
        .bind(name)
        .execute(&self.pool)
        .await
        .context("Failed to update resource")?;

        let stored = StoredObject {
            namespace: namespace.to_string(),
            kind: kind.to_string(),
            name: name.to_string(),
            object: object.clone(),
            uid: existing.uid,
            resource_version,
            generation,
            created_at: existing.created_at,
            updated_at: now,
        };

        let event = WatchEvent {
            event_type: WatchEventType::Modified,
            object,
            rev: resource_version.to_string(),
        };

        Ok((stored, event))
    }

    /// Delete a resource
    pub async fn delete(
        &self,
        namespace: &str,
        kind: &str,
        name: &str,
    ) -> Result<Option<WatchEvent>> {
        let existing = self.get(namespace, kind, name).await?;

        if existing.is_none() {
            return Ok(None);
        }

        let existing = existing.unwrap();
        let resource_version = self.next_revision().await?;

        sqlx::query(
            r#"
            DELETE FROM resources
            WHERE namespace = ? AND kind = ? AND name = ?
            "#,
        )
        .bind(namespace)
        .bind(kind)
        .bind(name)
        .execute(&self.pool)
        .await
        .context("Failed to delete resource")?;

        Ok(Some(WatchEvent {
            event_type: WatchEventType::Deleted,
            object: existing.object,
            rev: resource_version.to_string(),
        }))
    }
}
