use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum AddonType {
    Aircraft,
    Scenery,
    Plugin,
    Navdata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallTask {
    pub id: String,
    #[serde(rename = "type")]
    pub addon_type: AddonType,
    pub source_path: String,
    pub target_path: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_exists: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub tasks: Vec<InstallTask>,
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct NavdataCycle {
    pub name: String,
    #[serde(default)]
    pub cycle: Option<String>,
    #[serde(default)]
    pub airac: Option<String>,
}

#[derive(Debug)]
pub struct DetectedItem {
    pub addon_type: AddonType,
    pub path: String,
    pub display_name: String,
}
