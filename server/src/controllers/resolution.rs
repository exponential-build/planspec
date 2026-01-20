//! Plan resolution controller - determines which Plan satisfies a Goal.
//!
//! Resolution algorithm:
//! 1. Find Plans where `spec.goalRef.name` matches the Goal
//! 2. Filter by `planSelector` labels if specified on Goal
//! 3. Group by `spec.series`
//! 4. Within each series, select highest `spec.version`
//! 5. Exclude Plans that are superseded by others
//! 6. Set `Goal.status.activePlanRef` to selected Plan

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

use crate::storage::Store;
use crate::watch::WatchBroadcaster;

/// Controller that resolves Plans for Goals
pub struct PlanResolver {
    store: Store,
    broadcaster: WatchBroadcaster,
}

impl PlanResolver {
    /// Create a new PlanResolver
    pub fn new(store: Store, broadcaster: WatchBroadcaster) -> Self {
        Self { store, broadcaster }
    }

    /// Run resolution for all Goals in a namespace
    #[allow(dead_code)]
    pub async fn reconcile_namespace(&self, namespace: &str) -> Result<()> {
        debug!(namespace, "Reconciling goals in namespace");

        // Get all Goals in namespace
        let goals = self.store.list(namespace, "Goal").await?;
        if goals.is_empty() {
            return Ok(());
        }

        // Get all Plans in namespace
        let plans = self.store.list(namespace, "Plan").await?;

        for goal in goals {
            if let Err(e) = self.reconcile_goal(&goal.object, &plans).await {
                warn!(
                    goal = goal.name,
                    namespace,
                    error = %e,
                    "Failed to reconcile goal"
                );
            }
        }

        Ok(())
    }

    /// Run resolution for all Goals across all namespaces
    pub async fn reconcile_all(&self) -> Result<()> {
        debug!("Reconciling all goals");

        // Get all Goals
        let goals = self.store.list_all("Goal").await?;
        if goals.is_empty() {
            return Ok(());
        }

        // Get all Plans
        let plans = self.store.list_all("Plan").await?;

        // Group goals by namespace for efficiency
        let mut goals_by_ns: HashMap<String, Vec<Value>> = HashMap::new();
        for goal in goals {
            goals_by_ns
                .entry(goal.namespace.clone())
                .or_default()
                .push(goal.object);
        }

        // Group plans by namespace
        let mut plans_by_ns: HashMap<String, Vec<Value>> = HashMap::new();
        for plan in plans {
            plans_by_ns
                .entry(plan.namespace.clone())
                .or_default()
                .push(plan.object);
        }

        // Reconcile each namespace
        for (ns, ns_goals) in goals_by_ns {
            let ns_plans: Vec<_> = plans_by_ns
                .get(&ns)
                .map(|p| {
                    p.iter()
                        .map(|v| crate::storage::StoredObject {
                            namespace: ns.clone(),
                            kind: "Plan".to_string(),
                            name: v
                                .get("metadata")
                                .and_then(|m| m.get("name"))
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string(),
                            object: v.clone(),
                            uid: String::new(),
                            resource_version: 0,
                            generation: 0,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            for goal in ns_goals {
                if let Err(e) = self.reconcile_goal(&goal, &ns_plans).await {
                    let goal_name = goal
                        .get("metadata")
                        .and_then(|m| m.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown");
                    warn!(
                        goal = goal_name,
                        namespace = %ns,
                        error = %e,
                        "Failed to reconcile goal"
                    );
                }
            }
        }

        Ok(())
    }

    /// Reconcile a single Goal - find and set its active Plan
    async fn reconcile_goal(
        &self,
        goal: &Value,
        plans: &[crate::storage::StoredObject],
    ) -> Result<()> {
        let goal_name = goal
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow::anyhow!("Goal missing metadata.name"))?;

        let namespace = goal
            .get("metadata")
            .and_then(|m| m.get("namespace"))
            .and_then(|n| n.as_str())
            .unwrap_or("default");

        debug!(goal = goal_name, namespace, "Reconciling goal");

        // Get current activePlanRef
        let current_plan_ref = goal
            .get("status")
            .and_then(|s| s.get("activePlanRef"))
            .and_then(|r| r.get("name"))
            .and_then(|n| n.as_str());

        // Step 1: Find Plans that reference this Goal
        let matching_plans: Vec<_> = plans
            .iter()
            .filter(|p| {
                let goal_ref = p
                    .object
                    .get("spec")
                    .and_then(|s| s.get("goalRef"))
                    .and_then(|gr| gr.get("name"))
                    .and_then(|n| n.as_str());
                goal_ref == Some(goal_name)
            })
            .collect();

        if matching_plans.is_empty() {
            debug!(goal = goal_name, "No plans found for goal");
            // No plans found - set phase to Pending if not already set
            if current_plan_ref.is_some() {
                // Had a plan but now none - update status
                self.update_goal_status(namespace, goal_name, None).await?;
            }
            return Ok(());
        }

        // Step 2: Filter by planSelector if specified
        let plan_selector = goal.get("spec").and_then(|s| s.get("planSelector"));
        let filtered_plans: Vec<_> = if let Some(selector) = plan_selector {
            let match_labels = selector
                .get("matchLabels")
                .and_then(|m| m.as_object())
                .map(|o| {
                    o.iter()
                        .map(|(k, v)| (k.as_str(), v.as_str().unwrap_or("")))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            matching_plans
                .into_iter()
                .filter(|p| {
                    let labels = p
                        .object
                        .get("metadata")
                        .and_then(|m| m.get("labels"))
                        .and_then(|l| l.as_object());

                    match_labels.iter().all(|(k, v)| {
                        labels.and_then(|l| l.get(*k)).and_then(|lv| lv.as_str()) == Some(*v)
                    })
                })
                .collect()
        } else {
            matching_plans
        };

        if filtered_plans.is_empty() {
            debug!(goal = goal_name, "No plans match selector for goal");
            return Ok(());
        }

        // Step 3: Group by series
        let mut by_series: HashMap<Option<String>, Vec<_>> = HashMap::new();
        for plan in &filtered_plans {
            let series = plan
                .object
                .get("spec")
                .and_then(|s| s.get("series"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            by_series.entry(series).or_default().push(plan);
        }

        // Step 4: Within each series, select highest version
        let mut candidates: Vec<&crate::storage::StoredObject> = Vec::new();
        for (_series, series_plans) in by_series {
            let best = series_plans.into_iter().max_by(|a, b| {
                let v_a = a
                    .object
                    .get("spec")
                    .and_then(|s| s.get("version"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                let v_b = b
                    .object
                    .get("spec")
                    .and_then(|s| s.get("version"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("0");
                compare_versions(v_a, v_b)
            });
            if let Some(p) = best {
                candidates.push(p);
            }
        }

        // Step 5: Exclude superseded plans
        let superseded: HashSet<String> = candidates
            .iter()
            .flat_map(|p| {
                p.object
                    .get("spec")
                    .and_then(|s| s.get("supersedes"))
                    .and_then(|ss| ss.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|r| r.get("name").and_then(|n| n.as_str()))
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();

        let final_candidates: Vec<_> = candidates
            .into_iter()
            .filter(|p| !superseded.contains(&p.name))
            .collect();

        // Select the best plan (highest priority/version among candidates)
        let selected = final_candidates.into_iter().max_by(|a, b| {
            let v_a = a
                .object
                .get("spec")
                .and_then(|s| s.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("0");
            let v_b = b
                .object
                .get("spec")
                .and_then(|s| s.get("version"))
                .and_then(|v| v.as_str())
                .unwrap_or("0");
            compare_versions(v_a, v_b)
        });

        // Step 6: Update Goal.status.activePlanRef
        if let Some(plan) = selected {
            let plan_name = &plan.name;
            if current_plan_ref != Some(plan_name.as_str()) {
                info!(
                    goal = goal_name,
                    plan = %plan_name,
                    namespace,
                    "Resolved plan for goal"
                );
                self.update_goal_status(namespace, goal_name, Some(plan_name))
                    .await?;
            }
        }

        Ok(())
    }

    /// Update Goal status with the resolved plan reference
    async fn update_goal_status(
        &self,
        namespace: &str,
        goal_name: &str,
        plan_name: Option<&str>,
    ) -> Result<()> {
        // Get current goal
        let goal = self
            .store
            .get(namespace, "Goal", goal_name)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Goal not found"))?;

        let mut updated = goal.object.clone();

        // Get current generation for observedGeneration
        let generation = updated
            .get("metadata")
            .and_then(|m| m.get("generation"))
            .and_then(|g| g.as_i64())
            .unwrap_or(1);

        let now = chrono::Utc::now().to_rfc3339();

        // Initialize status if not present
        if updated.get("status").is_none() {
            updated["status"] = json!({});
        }

        // Set observedGeneration
        updated["status"]["observedGeneration"] = json!(generation);

        // Get or create conditions array
        let mut conditions = updated
            .get("status")
            .and_then(|s| s.get("conditions"))
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        // Set activePlanRef and phase
        if let Some(plan) = plan_name {
            updated["status"]["activePlanRef"] = json!({
                "name": plan,
                "namespace": namespace
            });
            updated["status"]["phase"] = json!("Ready");

            // Update or add PlanResolved condition
            update_condition(
                &mut conditions,
                "PlanResolved",
                "True",
                "PlanFound",
                &format!("Plan '{}' resolved for goal", plan),
                &now,
                generation,
            );
        } else {
            // Remove activePlanRef
            if let Some(status) = updated.get_mut("status").and_then(|s| s.as_object_mut()) {
                status.remove("activePlanRef");
            }
            updated["status"]["phase"] = json!("Pending");

            // Update or add PlanResolved condition
            update_condition(
                &mut conditions,
                "PlanResolved",
                "False",
                "NoPlanFound",
                "No matching plan found for goal",
                &now,
                generation,
            );
        }

        updated["status"]["conditions"] = json!(conditions);

        // Update in store
        let (_, event) = self
            .store
            .replace(namespace, "Goal", goal_name, updated, None)
            .await?;

        // Broadcast change
        self.broadcaster.send(event);

        Ok(())
    }
}

/// Update or add a condition in the conditions array
fn update_condition(
    conditions: &mut Vec<Value>,
    condition_type: &str,
    status: &str,
    reason: &str,
    message: &str,
    timestamp: &str,
    generation: i64,
) {
    let new_condition = json!({
        "type": condition_type,
        "status": status,
        "reason": reason,
        "message": message,
        "lastTransitionTime": timestamp,
        "observedGeneration": generation
    });

    // Find existing condition of this type
    if let Some(pos) = conditions
        .iter()
        .position(|c| c.get("type").and_then(|t| t.as_str()) == Some(condition_type))
    {
        // Check if status changed
        let old_status = conditions[pos].get("status").and_then(|s| s.as_str());
        if old_status != Some(status) {
            // Status changed - update lastTransitionTime
            conditions[pos] = new_condition;
        } else {
            // Status unchanged - only update message/reason/observedGeneration
            conditions[pos]["reason"] = json!(reason);
            conditions[pos]["message"] = json!(message);
            conditions[pos]["observedGeneration"] = json!(generation);
        }
    } else {
        // Add new condition
        conditions.push(new_condition);
    }
}

/// Compare version strings (semantic or numeric)
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    // Try numeric comparison first
    if let (Ok(na), Ok(nb)) = (a.parse::<i64>(), b.parse::<i64>()) {
        return na.cmp(&nb);
    }

    // Try semver-style comparison (split by .)
    let parts_a: Vec<i64> = a.split('.').filter_map(|p| p.parse().ok()).collect();
    let parts_b: Vec<i64> = b.split('.').filter_map(|p| p.parse().ok()).collect();

    for (pa, pb) in parts_a.iter().zip(parts_b.iter()) {
        match pa.cmp(pb) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    // If all compared parts are equal, longer version is greater
    parts_a.len().cmp(&parts_b.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("1", "2"), std::cmp::Ordering::Less);
        assert_eq!(compare_versions("10", "2"), std::cmp::Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.1"), std::cmp::Ordering::Less);
        assert_eq!(
            compare_versions("1.1.0", "1.1"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(compare_versions("2.0", "1.9"), std::cmp::Ordering::Greater);
    }
}
