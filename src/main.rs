use std::process::Command;
use crate::model::{KubectlVersionResponse, AWSRssFeedResponse, Item};

mod model;

fn main() {
    let server_ver = get_server_k8s_version();
    let latest_eks_version = get_latest_eks_k8s_version();
    println!("server: {:?}", server_ver);
    println!("latest EKS: {:?}", latest_eks_version);

    if latest_eks_version > server_ver {
        println!("Server version {} is outdated", server_ver.to_string());
    }

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