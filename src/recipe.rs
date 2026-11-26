use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const REQUIRED: &[(&str, &str)] = &[
    ("ubuntu", "autoinstall"),
    ("debian", "preseed"),
    ("fedora", "kickstart"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallRecipe {
    pub schema_version: String,
    pub catalog_entry_id: String,
    pub mechanism: String,
    pub stability: String,
    /// Always false. The catalog records answers and does not run them.
    pub executed: bool,
    pub answers: BTreeMap<String, Value>,
    pub notes: String,
}

pub struct RecipeSubject<'a> {
    pub id: &'a str,
    pub mechanisms: Vec<(&'a str, &'a str)>,
}

pub fn recipe_problems(root: &Path, subjects: &[RecipeSubject<'_>]) -> Vec<String> {
    let mut problems = Vec::new();
    let dir = root.join("recipes");
    for subject in subjects {
        let required = REQUIRED
            .iter()
            .find(|(id, mechanism)| {
                *id == subject.id
                    && subject
                        .mechanisms
                        .iter()
                        .any(|(name, stability)| name == mechanism && *stability == "stable")
            })
            .copied();
        let path = dir.join(format!("{}.json", subject.id));
        if !path.is_file() {
            if let Some((_, mechanism)) = required {
                problems.push(format!(
                    "{} needs a declarative {mechanism} recipe",
                    subject.id
                ));
            }
            continue;
        }
        if let Err(err) = validate_recipe_file(&path, subject) {
            problems.push(err.to_string());
        }
    }
    if dir.is_dir()
        && let Ok(entries) = fs::read_dir(&dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name == ".gitkeep" {
                continue;
            }
            let Some(id) = name.strip_suffix(".json") else {
                problems.push(format!("{name} is not a declarative recipe"));
                continue;
            };
            if !subjects.iter().any(|subject| subject.id == id) {
                problems.push(format!("recipe {id} does not match a catalog entry"));
            }
        }
    }
    problems
}

pub fn load_recipe(root: &Path, id: &str) -> Result<Option<InstallRecipe>> {
    let path = root.join("recipes").join(format!("{id}.json"));
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|err| crate::error::io_at(&path, err))?;
    reject_text(&path, &text)?;
    let recipe: InstallRecipe = serde_json::from_str(&text)
        .map_err(|err| Error::msg(format!("{}: {err}", path.display())))?;
    if recipe.catalog_entry_id != id {
        return Err(Error::msg(format!(
            "{} catalog_entry_id does not match {id}",
            path.display()
        )));
    }
    if recipe.executed {
        return Err(Error::msg(format!("{} sets executed true", path.display())));
    }
    reject_answers(&recipe.answers)?;
    Ok(Some(recipe))
}

fn validate_recipe_file(path: &Path, subject: &RecipeSubject<'_>) -> Result<()> {
    let root = path.parent().and_then(Path::parent).ok_or_else(|| {
        Error::msg(format!(
            "{} is not inside a catalog recipes directory",
            path.display()
        ))
    })?;
    let Some(recipe) = load_recipe(root, subject.id)? else {
        return Err(Error::msg(format!("{} is missing", path.display())));
    };
    if recipe.schema_version != "1.0" {
        return Err(Error::msg(format!(
            "{} schema_version must be 1.0",
            path.display()
        )));
    }
    if recipe.notes.trim().is_empty() {
        return Err(Error::msg(format!("{} notes are required", path.display())));
    }
    let Some((_, stability)) = subject
        .mechanisms
        .iter()
        .find(|(name, _)| *name == recipe.mechanism)
    else {
        return Err(Error::msg(format!(
            "{} mechanism {} is not declared on the entry",
            path.display(),
            recipe.mechanism
        )));
    };
    if recipe.stability != *stability {
        return Err(Error::msg(format!(
            "{} stability does not match the catalog mechanism",
            path.display()
        )));
    }
    if recipe.stability == "unsupported" {
        return Err(Error::msg(format!(
            "{} cannot record an unsupported mechanism as an install recipe",
            path.display()
        )));
    }
    Ok(())
}

fn reject_answers(answers: &BTreeMap<String, Value>) -> Result<()> {
    for (key, value) in answers {
        reject_value(key, value)?;
    }
    Ok(())
}

fn reject_value(key: &str, value: &Value) -> Result<()> {
    let lowered = key.to_ascii_lowercase();
    if lowered.contains("command")
        || lowered.contains("post")
        || lowered.contains('%')
        || lowered.contains("script")
    {
        return Err(Error::msg(format!(
            "recipe answer {key} is not allowed in a declarative plan"
        )));
    }
    match value {
        Value::String(text) => reject_text(Path::new(key), text),
        Value::Array(items) => {
            for item in items {
                reject_value(key, item)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for (child, item) in map {
                reject_value(child, item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn reject_text(path: &Path, text: &str) -> Result<()> {
    let lowered = text.to_ascii_lowercase();
    for needle in [
        "#!/bin/",
        "#!/usr/bin/",
        "curl |",
        "wget |",
        "begin private key",
    ] {
        if lowered.contains(needle) {
            return Err(Error::msg(format!(
                "{} contains forbidden recipe text {needle}",
                path.display()
            )));
        }
    }
    Ok(())
}
