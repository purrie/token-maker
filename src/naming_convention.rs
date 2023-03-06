use std::collections::HashMap;

use crate::{
    data::sanitize_file_name,
    persistence::{Persistence, PersistentKey, PersistentValue},
    workspace::WorkspaceTemplate,
};

/// Structure holds information about default values for names used throughout the program
#[derive(Debug)]
pub struct NamingConvention {
    convention: HashMap<WorkspaceTemplate, String>,
    pub project_name: String,
}

enum PersistentData {
    NamingConventionID,
}

impl PersistentKey for PersistentData {
    fn get_id(&self) -> &'static str {
        match self {
            PersistentData::NamingConventionID => "naming-convention",
        }
    }
}

impl NamingConvention {
    pub const KEYWORD_PROJECT: &str = "$project_name";

    /// Constructs new naming convention, loading default values from the cache if present
    pub fn new(cache: &Persistence) -> Self {
        let mut convention = HashMap::new();
        // Inserting default or saved names for each template type
        WorkspaceTemplate::ALL.iter().for_each(|wt| {
            convention.insert(
                wt.clone(),
                match cache.get(PersistentData::NamingConventionID, wt.clone()) {
                    Some(n) => match n {
                        PersistentValue::String(n) => n.clone(),
                        _ => format!(
                            "{}{}",
                            NamingConvention::KEYWORD_PROJECT,
                            wt.get_default_file_name()
                        ),
                    },
                    None => {
                        format!(
                            "{}{}",
                            NamingConvention::KEYWORD_PROJECT,
                            wt.get_default_file_name()
                        )
                    }
                },
            );
        });
        Self {
            convention,
            project_name: String::from(""),
        }
    }
    /// Returns a copy of the naming convention for specified template
    pub fn get(&self, template: &WorkspaceTemplate) -> String {
        self.convention.get(template).unwrap().clone()
    }
    /// Borrows the naming convention for specified template
    pub fn check(&self, template: &WorkspaceTemplate) -> &str {
        self.convention.get(template).unwrap()
    }
    /// Sets naming convention for specified template, saving it to cache as well
    pub fn set(&mut self, template: WorkspaceTemplate, name: String, cache: &mut Persistence) {
        let name = sanitize_file_name(name);
        cache.set(PersistentData::NamingConventionID, template, name.clone());
        self.convention.insert(template, name);
    }
}
