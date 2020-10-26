use crate::model::{KubectlVersionResponse, AWSRssFeedResponse, Item, State};
use std::net::SocketAddr;
use prometheus_exporter::prometheus::{register_gauge, Opts, register_gauge_vec, GaugeVec};

extern crate prometheus_exporter;

mod model;

fn main() {
    let mut state = State::new();
    println!("server: {:?}", state.server_ver);
    println!("latest EKS: {:?}", state.latest_eks_version);
    println!("latest k8s release: {:?}", state.latest_k8s_version);
    println!("oldest k8s release supported: {:?}", state.eol_k8s_version);

    if state.latest_eks_version > state.server_ver {
        println!("Server version {} is outdated", state.server_ver.to_string());
    }

    let server_current_version_vec = register_gauge_vec!("eks_version_exporter", "Bunch of values", &["server_current_version", "eks_latest_available_version", "k8s_latest_available_version", "eol_latest_available_version", "last_updated", "last_updated_text", "is_outdated"]).expect("Unable to create gauge vec");
    server_current_version_vec.with_label_values(&[
        state.server_ver.to_string().as_str(),
        state.latest_eks_version.to_string().as_str(),
        state.latest_k8s_version.to_string().as_str(),
        state.eol_k8s_version.to_string().as_str(),
        &state.current_time,
        &state.current_time_date_string,
        state.is_outdated.to_string().as_str()]);


    let addr_raw = "0.0.0.0:8080";
    let addr : SocketAddr = addr_raw.parse().expect("Invalid SocketAddr");
    let (req_recv, fin_send) = prometheus_exporter::PrometheusExporter::run_and_repeat(addr, std::time::Duration::from_secs(1800));

    loop {
        req_recv.recv().unwrap();

        println!("{}: Updating metrics", model::current_time_date_string());
        state.refresh();

        if state.latest_eks_version > state.server_ver {
            println!("{}: Server version {} is outdated", model::current_time_date_string(), state.server_ver.to_string());
        }


        server_current_version_vec.reset();

        server_current_version_vec.with_label_values(&[
            state.server_ver.to_string().as_str(),
            state.latest_eks_version.to_string().as_str(),
            state.latest_k8s_version.to_string().as_str(),
            state.eol_k8s_version.to_string().as_str(),
            &state.current_time,
            &state.current_time_date_string,
            state.is_outdated.to_string().as_str()]);

        fin_send.send(prometheus_exporter::FinishedUpdate).unwrap();
    }
}
