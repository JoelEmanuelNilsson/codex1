//! DAG types and builder.
//!
//! A [`Dag`] is constructed by passing a parsed [`Blueprint`] through
//! [`validate::build_dag`], which enforces id-format, uniqueness,
//! dependency-existence, and acyclicity rules. Consumers
//! (`graph::waves`, `status`, `cli::plan`) only see a validated DAG.

// T8 (waves), T9 (status), T11/T12 (CLI commands) will consume these types.
#![allow(dead_code)]

pub(crate) mod validate;
pub(crate) mod waves;

use std::collections::BTreeMap;

use crate::blueprint::{Blueprint, TaskSpec};

/// Validated plan DAG. Task order is determined by the `BTreeMap` sort on
/// task id so iteration is deterministic across runs.
#[derive(Debug, Clone)]
pub struct Dag {
    pub graph_revision: u64,
    pub tasks: BTreeMap<String, TaskSpec>,
}

impl Dag {
    /// All task IDs in sorted order.
    #[must_use]
    pub fn ids(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    /// Number of tasks.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Whether the DAG is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Lookup a task by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&TaskSpec> {
        self.tasks.get(id)
    }

    /// Direct dependencies of `id`, or an empty slice if the task is missing.
    #[must_use]
    pub fn deps_of<'a>(&'a self, id: &str) -> &'a [String] {
        self.tasks
            .get(id)
            .map_or(&[][..], |t| t.depends_on.as_slice())
    }
}

/// Public convenience: validate and build a DAG from a parsed blueprint.
pub fn build_dag(blueprint: &Blueprint) -> Result<Dag, crate::error::CliError> {
    validate::build_dag(blueprint)
}
