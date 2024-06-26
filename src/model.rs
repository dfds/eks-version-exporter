use serde::{Serialize, Deserialize};
use github_rss::GithubFeedResponse;
use std::process::Command;
use log::{debug, info};
use std::error::Error;
use regex::Regex;
use reqwest::header::HeaderValue;
use reqwest::Url;

pub struct State {
    pub server_ver : semver::Version,
    pub latest_eks_version : semver::Version,
    pub latest_k8s_version : semver::Version,
    pub eol_k8s_version : semver::Version,
    pub current_time : String,
    pub current_time_date_string : String,
    pub is_outdated : f64,
    pub is_past_eol: f64
}

impl State {
    pub fn new() -> Self {
        let mut state = State {
            server_ver: semver::Version {
                major: 0,
                minor: 0,
                patch: 0,
                pre: vec![],
                build: vec![]
            },
            latest_eks_version: semver::Version {
                major: 0,
                minor: 0,
                patch: 0,
                pre: vec![],
                build: vec![]
            },
            latest_k8s_version: semver::Version {
                major: 0,
                minor: 0,
                patch: 0,
                pre: vec![],
                build: vec![]
            },
            eol_k8s_version: semver::Version {
                major: 0,
                minor: 0,
                patch: 0,
                pre: vec![],
                build: vec![]
            },
            current_time: "".to_string(),
            current_time_date_string: "".to_string(),
            is_outdated: 0.0,
            is_past_eol: 0.0
        };

        state.refresh();
        state
    }

    pub fn refresh(&mut self) {
        self.server_ver = get_server_k8s_version();
        self.latest_eks_version =  get_latest_eks_k8s_version();
        self.latest_k8s_version =  get_latest_k8s_version().expect("Unable to get k8s_releases");
        self.eol_k8s_version =  {
            let mut x = self.latest_k8s_version.clone();
            x.minor = x.minor - 2;
            x.patch = 0;
            x
        };
        self.current_time =  current_time_epoch().to_string();
        self.current_time_date_string =  current_time_date_string();

        if self.latest_eks_version > self.server_ver {
            info!(target: "eks_version_exporter", "Server is outdated: {} > {}", self.latest_eks_version, self.server_ver);
            self.is_outdated = 1.0;
        } else {
            self.is_outdated = 0.0;
        }

        // 1.17.4 > 1.17.0
        if self.eol_k8s_version > self.server_ver {
            info!(target: "eks_version_exporter", "Server is EOL: {} >= {}", self.eol_k8s_version, self.server_ver);
            self.is_past_eol = 1.0;
        } else {
            self.is_past_eol = 0.0;
        }
    }
}


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

    #[serde(rename = "item", default)]
    pub items: Vec<Item>
}

#[derive(Debug, Deserialize, Clone)]
pub struct Item {
    pub title : String,
    pub link : String
}

pub mod github_rss {
    use serde::{Serialize, Deserialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct GithubFeedResponse {
        pub updated : String,

        #[serde(rename = "entry", default)]
        pub entrys : Vec<Entry>
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Entry {
        pub title : String,
        pub id : String
    }
}

//

pub fn current_time_epoch() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    since_epoch.as_millis()
}

pub fn current_time_date_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    use chrono::prelude::DateTime;
    use chrono::Utc;

    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    let d = UNIX_EPOCH + since_epoch;

    let datetime = DateTime::<Utc>::from(d);
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn get_latest_eks_k8s_version() -> semver::Version {
    use regex::Regex;
    let res = get_aws_k8s_versions().unwrap();
    let re = Regex::new(r"(Kubernetes version \d+.[0-9]+)").unwrap();

    let version_items : Vec<Item> = res.channel.items
        .clone()
        .into_iter()
        .filter(|item| {
            re.is_match(&item.title)
        })
        .map(|mut item| {
            let caps = re.captures(&item.title).unwrap();
            item.title = caps.get(1).unwrap().as_str().to_owned();
            item
        })
        .collect();

    let mut parsed_versions = Vec::new();
    for i in &version_items {
        let item = i.clone();
        let last = item.title.split(' ').last().expect("");
        let mut modified : String = "".to_owned();
        if last.split('.').collect::<Vec<&str>>().len() >= 2 {
            modified = format!("{}.0", last);
        } else {
            modified = last.to_owned();
        }
        match semver::Version::parse(&modified) {
            Ok(ver) => parsed_versions.push(ver),
            Err(err) => {
                debug!("Unable to parse item title into Version, skipping");
                debug!("{}", err);
            }
        }
    }

    parsed_versions.sort_by_key(|x| x.minor);

   parsed_versions.last().unwrap().clone()
}

pub fn get_server_k8s_version() -> semver::Version {
    let versions = get_k8s_version().expect("Unable to get server version");

    let server_ver_raw = versions.server_version.as_ref().expect("No server version").git_version.as_ref().expect("No git version key");
    let server_ver_modified : &str = server_ver_raw.chars().next().map(|c| &server_ver_raw[c.len_utf8()..]).expect("Unable to remove first character");
    let server_ver = semver::Version::parse(server_ver_modified).expect("Unable to parse git version into Version");
    semver::Version::parse(format!("{}.{}.0", server_ver.major, server_ver.minor).as_str()).expect("Unable to parse git version into Version")
}

pub fn get_k8s_version() -> Result<KubectlVersionResponse, ()> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("kubectl version -ojson")
        .output()
        .expect("Unable to get output");

    let output_str = std::str::from_utf8(&output.stdout).expect("Unable to convert output to UTF8 str");

    return match serde_json::from_str::<KubectlVersionResponse>(output_str) {
        Ok(val) => Ok(val),
        Err(_) => Err(())
    };
}

fn get_aws_k8s_versions() -> Result<AWSRssFeedResponse, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();

    let mut req = reqwest::blocking::Request::new(reqwest::Method::GET, Url::parse("https://docs.aws.amazon.com/eks/latest/userguide/doc-history.rss").unwrap());
    req.headers_mut().insert("User-Agent", HeaderValue::from_str("curl/7.77.0").unwrap()); // CloudFront in front of the RSS feed does some user-agent checking now

    let resp = match client.execute(req) {
        Ok(val) => val,
        Err(err) => {
            return Err(Box::new(err));
        }
    };
    
    let resp_text = match resp.text() {
        Ok(val) => val,
        Err(err) => {
            return Err(Box::new(err));
        }
    };

    return match serde_xml_rs::from_str(&resp_text) {
        Ok(val) => Ok(val),
        Err(err) => Err(Box::new(err))
    }
}

fn get_k8s_releases() -> Result<GithubFeedResponse, ()> {
    let resp = reqwest::blocking::get("https://github.com/kubernetes/kubernetes/releases.atom").expect("Unable to get RSS response from AWS");
    let resp_text = resp.text().expect("Unable to convert HTTP response to text");

    return match serde_xml_rs::from_str(&resp_text) {
        Ok(val) => Ok(val),
        Err(_) => Err(())
    }
}

fn get_latest_k8s_version() -> Option<semver::Version> {
    let k8s_releases = get_k8s_releases().expect("Unable to get k8s_releases");
    let mut releases = Vec::new();
    for release in &k8s_releases.entrys {
        let tag = release.id.clone();
        let version_from_tag = tag.split('/').last().expect("Unable to split tag by char '/'. RSS feed structure must've changed");

        let modified = version_from_tag.chars().next().map(|c| &version_from_tag[c.len_utf8()..]).expect("Unable to remove first character");
        let version = semver::Version::parse(modified).expect("Unable to parse version");
        releases.push(version);
    }

    releases = releases
        .into_iter()
        .filter(|release| {
            return release.pre.is_empty();
        })
        .collect::<Vec<semver::Version>>();

    releases.sort_by_key(|x| x.minor);

    match releases.last() {
        Some(v) => Some(v.clone()),
        None => None
    }
}
