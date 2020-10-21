use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KubectlVersionResponse {
    #[serde(rename = "clientVersion")]
    pub client_version: Option<Version>,
    #[serde(rename = "serverVersion")]
    pub server_version: Option<Version>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Version {
    pub major: Option<String>,
    pub minor: Option<String>,
    #[serde(rename = "gitVersion")]
    pub git_version: Option<String>,
    #[serde(rename = "gitCommit")]
    pub git_commit: Option<String>,
    #[serde(rename = "gitTreeState")]
    pub git_tree_state: Option<String>,
    #[serde(rename = "buildDate")]
    pub build_date: Option<String>,
    #[serde(rename = "goVersion")]
    pub go_version: Option<String>,
    pub compiler: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AWSRssFeedResponse {
    pub channel : Channel
}

#[derive(Debug, Deserialize)]
pub struct Channel {
    pub title : String,
    #[serde(rename = "lastBuildDate")]
    pub last_build_date : String,

    #[serde(rename = "item", default)]
    pub items: Vec<Item>
}

#[derive(Debug, Deserialize, Clone)]
pub struct Item {
    pub title : String,
    pub link : String
}