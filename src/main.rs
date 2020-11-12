use crate::model::{State};
use std::net::SocketAddr;
use prometheus_exporter::prometheus::{register_gauge, register_gauge_vec};
use log::{info};

extern crate prometheus_exporter;

mod model;

fn main() {
    setup();
    let mut state = State::new();
    info!("server: {:?}", state.server_ver);
    info!("latest EKS: {:?}", state.latest_eks_version);
    info!("latest k8s release: {:?}", state.latest_k8s_version);
    info!("oldest k8s release supported: {:?}", state.eol_k8s_version);

    if state.latest_eks_version > state.server_ver {
        info!("Server version {} is outdated", state.server_ver.to_string());
    }

    let server_current_version_vec = register_gauge_vec!("eks_version_exporter", "Bunch of values", &["server_current_version", "eks_latest_available_version", "k8s_latest_available_version", "eol_latest_available_version", "last_updated", "last_updated_text"]).expect("Unable to create gauge vec");
    server_current_version_vec.with_label_values(&[
        state.server_ver.to_string().as_str(),
        state.latest_eks_version.to_string().as_str(),
        state.latest_k8s_version.to_string().as_str(),
        state.eol_k8s_version.to_string().as_str(),
        &state.current_time,
        &state.current_time_date_string]);

    let is_outdated_gauge = register_gauge!("eks_version_exporter_is_outdated", "If value is 1 then cluster version is outdated").expect("Unable to create gauge");
    is_outdated_gauge.set(state.is_outdated);

    let is_past_eol_gauge = register_gauge!("eks_version_exporter_is_past_eol", "If value is 1 then cluster version is older than EOL").expect("Unable to create gauge");
    is_past_eol_gauge.set(state.is_past_eol);


    let addr_raw = "0.0.0.0:8080";
    let addr : SocketAddr = addr_raw.parse().expect("Invalid SocketAddr");
    let (req_recv, fin_send) = prometheus_exporter::PrometheusExporter::run_and_repeat(addr, std::time::Duration::from_secs(1800));

    loop {
        req_recv.recv().unwrap();

        info!("{}: Updating metrics", model::current_time_date_string());
        state.refresh();

        info!("server: {:?}", state.server_ver);
        info!("latest EKS: {:?}", state.latest_eks_version);
        info!("latest k8s release: {:?}", state.latest_k8s_version);
        info!("oldest k8s release supported: {:?}", state.eol_k8s_version);

        server_current_version_vec.reset();

        server_current_version_vec.with_label_values(&[
            state.server_ver.to_string().as_str(),
            state.latest_eks_version.to_string().as_str(),
            state.latest_k8s_version.to_string().as_str(),
            state.eol_k8s_version.to_string().as_str(),
            &state.current_time,
            &state.current_time_date_string]);

        is_outdated_gauge.set(state.is_outdated);
        is_past_eol_gauge.set(state.is_past_eol);

        fin_send.send(prometheus_exporter::FinishedUpdate).unwrap();
    }
}

fn setup() {
    std::env::set_var("RUST_LOG", "trace,mio=info,hyper=info,actix_server=debug,actix_http=debug,tokio_reactor=info,tokio_threadpool=info,serde_xml_rs=info,reqwest=info,want=info,tracing=info");
    pretty_env_logger::init_timed();
}