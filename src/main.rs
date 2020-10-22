use std::process::Command;
use crate::model::{KubectlVersionResponse, AWSRssFeedResponse, Item};
use std::net::SocketAddr;
use prometheus_exporter::prometheus::{register_gauge, Opts, register_gauge_vec, GaugeVec};
use crate::model::github_rss::GithubFeedResponse;

extern crate prometheus_exporter;

mod model;

fn main() {
    let mut latest_k8s_version = get_latest_k8s_version().expect("Unable to get k8s_releases");
    let mut eol_k8s_version = latest_k8s_version.clone();
    eol_k8s_version.minor = eol_k8s_version.minor - 2;
    let mut server_ver = get_server_k8s_version();
    let mut latest_eks_version = get_latest_eks_k8s_version();
    println!("server: {:?}", server_ver);
    println!("latest EKS: {:?}", latest_eks_version);
    println!("latest k8s release: {:?}", latest_k8s_version);
    println!("oldest k8s release supported: {:?}", eol_k8s_version);

    if latest_eks_version > server_ver {
        println!("Server version {} is outdated", server_ver.to_string());
    }


    let gauge = prometheus_exporter::prometheus::Gauge::new("eks_version_exporter_server_current_version", "Contains a semver compatible value").expect("Unable to create gauge");

    let mut opts = Opts::new("eks_version_exporter", "Bunch of values");
    opts = opts.const_label("server_current_version", server_ver.to_string().as_str());
    opts = opts.const_label("eks_latest_available_version", latest_eks_version.to_string().as_str());
    opts = opts.const_label("k8s_latest_available_version", latest_k8s_version.to_string().as_str());
    opts = opts.const_label("eol_latest_available_version", eol_k8s_version.to_string().as_str());
    opts = opts.const_label("last_updated", current_time_epoch().to_string().as_str());
    opts = opts.const_label("last_updated_text", current_time_date_string().as_str());
    let mut server_current_version = register_gauge!(opts).expect("Unable to register gauge");

    let server_current_version_vec = register_gauge_vec!("eks_version_exporter_vec", "Bunch of values", &["server_current_version", "eks_latest_available_version", "k8s_latest_available_version", "eol_latest_available_version", "last_updated", "last_updated_text"]).expect("Unable to create gauge vec");
    server_current_version_vec.with_label_values(&[
        server_ver.to_string().as_str(),
        latest_eks_version.to_string().as_str(),
        latest_k8s_version.to_string().as_str(),
        eol_k8s_version.to_string().as_str(),
        current_time_epoch().to_string().as_str(),
        current_time_date_string().as_str()]);


    let addr_raw = "0.0.0.0:8080";
    let addr : SocketAddr = addr_raw.parse().expect("Invalid SocketAddr");
    let (req_recv, fin_send) = prometheus_exporter::PrometheusExporter::run_and_repeat(addr, std::time::Duration::from_secs(60));

    loop {
        req_recv.recv().unwrap();

        println!("Updating metrics");

        latest_k8s_version = get_latest_k8s_version().expect("Unable to get k8s_releases");
        eol_k8s_version = latest_k8s_version.clone();
        eol_k8s_version.minor = eol_k8s_version.minor - 2;
        server_ver = get_server_k8s_version();
        latest_eks_version = get_latest_eks_k8s_version();

        prometheus_exporter::prometheus::unregister(Box::new(server_current_version));

        server_current_version_vec.reset();

        server_current_version_vec.with_label_values(&[
            server_ver.to_string().as_str(),
            latest_eks_version.to_string().as_str(),
            latest_k8s_version.to_string().as_str(),
            eol_k8s_version.to_string().as_str(),
            current_time_epoch().to_string().as_str(),
            current_time_date_string().as_str()]);

        let mut opts = Opts::new("eks_version_exporter", "Bunch of values");
        opts = opts.const_label("server_current_version", server_ver.to_string().as_str());
        opts = opts.const_label("eks_latest_available_version", latest_eks_version.to_string().as_str());
        opts = opts.const_label("k8s_latest_available_version", latest_k8s_version.to_string().as_str());
        opts = opts.const_label("eol_latest_available_version", eol_k8s_version.to_string().as_str());
        opts = opts.const_label("last_updated", current_time_epoch().to_string().as_str());
        opts = opts.const_label("last_updated_text", current_time_date_string().as_str());
        server_current_version = register_gauge!(opts).expect("Unable to register gauge");

        fin_send.send(prometheus_exporter::FinishedUpdate).unwrap();
    }
}

fn current_time_epoch() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let start = SystemTime::now();
    let since_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
    since_epoch.as_millis()
}

fn current_time_date_string() -> String {
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
    let res = get_aws_k8s_versions().expect("Unable to get response from AWS");
    let version_items : Vec<Item> = res.channel.items
        .clone()
        .into_iter()
        .filter(|item| {
            item.title.contains("Kubernetes version")
        })
        .collect();

    let item = version_items[0].clone();
    let last = item.title.split(' ').last().expect("");
    let mut modified : String = "".to_owned();
    if last.split('.').collect::<Vec<&str>>().len() >= 2 {
        modified = format!("{}.0", last);
    } else {
        modified = last.to_owned();
    }

    semver::Version::parse(&modified).expect("Unable to parse item title into Version")
}

fn get_server_k8s_version() -> semver::Version {
    let versions = get_k8s_version().expect("Unable to get server version");

    let server_ver_raw = versions.server_version.as_ref().expect("No server version").git_version.as_ref().expect("No git version key");
    let server_ver_modified : &str = server_ver_raw.chars().next().map(|c| &server_ver_raw[c.len_utf8()..]).expect("Unable to remove first character");
    let mut server_ver = semver::Version::parse(server_ver_modified).expect("Unable to parse git version into Version");
    semver::Version::parse(format!("{}.{}.0", server_ver.major, server_ver.minor).as_str()).expect("Unable to parse git version into Version")
}

fn get_k8s_version() -> Result<KubectlVersionResponse, ()> {
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

fn get_aws_k8s_versions() -> Result<AWSRssFeedResponse, ()> {
    let resp = reqwest::blocking::get("https://docs.aws.amazon.com/eks/latest/userguide/doc-history.rss").expect("Unable to get RSS response from AWS");
    let resp_text = resp.text().expect("Unable to convert HTTP response to text");

    return match serde_xml_rs::from_str(&resp_text) {
        Ok(val) => Ok(val),
        Err(_) => Err(())
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
        let modified = release.title.chars().next().map(|c| &release.title[c.len_utf8()..]).expect("Unable to remove first character");
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
