use chrono::Utc;
use reqwest::{Client as HttpClient, StatusCode};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::transport::Channel;
use tonic::{Request, Response, Status};
use zynk::engine::kv::LsmEngine;

pub mod pb {
    tonic::include_proto!("kv");
}

use pb::kv_client::KvClient;
use pb::{
    kv_server::{Kv, KvServer},
    DelRequest, DelResponse, GetRequest, GetResponse, PutRequest, PutResponse,
};

struct KvSvc {
    engine: Arc<RwLock<LsmEngine>>,
}

#[tonic::async_trait]
impl Kv for KvSvc {
    async fn put(&self, request: Request<PutRequest>) -> Result<Response<PutResponse>, Status> {
        let req = request.into_inner();
        let mut eng = self.engine.write().await;
        eng.put(&req.key, &req.value).map_err(to_status)?;
        Ok(Response::new(PutResponse {}))
    }

    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let req = request.into_inner();
        let eng = self.engine.read().await;
        match eng.get(&req.key).map_err(to_status)? {
            Some(v) => Ok(Response::new(GetResponse {
                value: v,
                found: true,
            })),
            None => Ok(Response::new(GetResponse {
                value: Vec::new(),
                found: false,
            })),
        }
    }

    async fn del(&self, request: Request<DelRequest>) -> Result<Response<DelResponse>, Status> {
        let req = request.into_inner();
        let mut eng = self.engine.write().await;
        eng.delete(&req.key).map_err(to_status)?;
        Ok(Response::new(DelResponse { removed: true }))
    }
}

fn get_or_create_actor_id(data_dir: &PathBuf) -> std::io::Result<u64> {
    let id_path = data_dir.join("actor_id");
    if id_path.exists() {
        let s = fs::read_to_string(&id_path)?;
        if let Ok(id) = s.trim().parse::<u64>() {
            return Ok(id);
        }
    }
    use rand::{thread_rng, Rng};
    let id: u64 = thread_rng().gen();
    fs::write(&id_path, id.to_string())?;
    Ok(id)
}

fn to_status(e: std::io::Error) -> Status {
    Status::internal(e.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50051);
    let bind_ip = std::env::var("BIND_IP").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr: SocketAddr = format!("{bind_ip}:{port}").parse()?;
    let data_dir = PathBuf::from(std::env::var("DATA_DIR").unwrap_or_else(|_| "/data".to_string()));
    let node_id = std::env::var("NODE_ID").unwrap_or_else(|_| "node-unknown".to_string());
    let peers_csv = std::env::var("PEERS").unwrap_or_default();
    let peers: Vec<String> = peers_csv
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // derive actor id for this node:
    let actor_id = get_or_create_actor_id(&data_dir)?;

    let engine = LsmEngine::new_with_manifest_and_actor(&data_dir, 64 * 1024, 8 * 1024, actor_id)?;
    let svc = KvSvc {
        engine: Arc::new(RwLock::new(engine)),
    };
    println!(
        "zynkd listening on {} (ACTOR_ID={}, DATA_DIR={})",
        addr,
        actor_id,
        data_dir.display()
    );

    let ns = std::env::var("NAMESPACE").unwrap_or_else(|_| {
        fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/namespace")
            .unwrap_or_else(|_| "default".to_string())
            .trim()
            .to_string()
    });
    let peers_for_task = peers.clone();
    let holder_id = node_id.clone();
    tokio::spawn(async move {
        println!(
            "[election] starting loop ns={} holder_id={} peers={:?}",
            ns, holder_id, peers_for_task
        );
        // Build HTTPS client with SA token and in-cluster CA
        let token = match fs::read_to_string("/var/run/secrets/kubernetes.io/serviceaccount/token") {
            Ok(t) => { println!("[election] SA token loaded"); t },
            Err(e) => { println!("[election] cannot read SA token: {}", e); return; }
        };
        let ca_bytes = match fs::read("/var/run/secrets/kubernetes.io/serviceaccount/ca.crt") {
            Ok(b) => { println!("[election] SA CA loaded"); b },
            Err(e) => { println!("[election] cannot read SA CA: {}", e); return; }
        };
        let cert = match reqwest::Certificate::from_pem(&ca_bytes) {
            Ok(c) => { println!("[election] CA parsed"); c },
            Err(e) => { println!("[election] invalid CA cert: {}", e); return; }
        };
        let http = match HttpClient::builder().add_root_certificate(cert).build() {
            Ok(c) => { println!("[election] http client built"); c },
            Err(e) => { println!("[election] http client build error: {}", e); return; }
        };
        let host = std::env::var("KUBERNETES_SERVICE_HOST").unwrap_or_else(|_| "kubernetes.default.svc".to_string());
        let port = std::env::var("KUBERNETES_SERVICE_PORT").unwrap_or_else(|_| "443".to_string());
        let base = format!("https://{}:{}", host, port);
        let lease_name = "zynk-coordinator";
        let lease_url = format!("{}/apis/coordination.k8s.io/v1/namespaces/{}/leases/{}", base, ns, lease_name);
        let list_url = format!("{}/apis/coordination.k8s.io/v1/namespaces/{}/leases", base, ns);
        let auth = format!("Bearer {}", token.trim());
        println!("[election] api base={} lease_url={}", base, lease_url);
        let ttl_secs = 15i64;
        loop {
            // Ensure lease exists
            let get_resp = http.get(&lease_url)
                .header(AUTHORIZATION, HeaderValue::from_str(&auth).unwrap())
                .send().await;
            let mut is_leader = false;
            if let Ok(resp) = get_resp {
                if resp.status() == StatusCode::NOT_FOUND {
                    println!("[election] lease not found, creating");
                    // create
                    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
                    let body = serde_json::json!({
                        "apiVersion": "coordination.k8s.io/v1",
                        "kind": "Lease",
                        "metadata": {"name": lease_name},
                        "spec": {
                            "holderIdentity": holder_id,
                            "leaseDurationSeconds": ttl_secs,
                            "acquireTime": now,
                            "renewTime": now
                        }
                    });
                    let res = http.post(&list_url)
                        .header(AUTHORIZATION, HeaderValue::from_str(&auth).unwrap())
                        .header(CONTENT_TYPE, "application/json")
                        .json(&body).send().await;
                    match res {
                        Ok(r) => println!("[election] create lease status={}", r.status()),
                        Err(e) => println!("[election] create lease error={}", e),
                    }
                }
            } else if let Err(e) = get_resp { println!("[election] get lease error={}", e); }

            // Try to renew/hold leadership with merge PATCH (avoid resourceVersion requirements)
            let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true);
            let patch = serde_json::json!({
                "spec": {
                    "holderIdentity": holder_id,
                    "leaseDurationSeconds": ttl_secs,
                    "renewTime": now
                }
            });
            match http.patch(&lease_url)
                .header(AUTHORIZATION, HeaderValue::from_str(&auth).unwrap())
                .header(CONTENT_TYPE, "application/merge-patch+json")
                .body(serde_json::to_vec(&patch).unwrap())
                .send().await {
                Ok(resp) => {
                    println!("[election] renew status={}", resp.status());
                    if resp.status().is_success() {
                        // inspect current holder
                        if let Ok(val) = resp.json::<serde_json::Value>().await {
                            let current = val.get("spec").and_then(|s| s.get("holderIdentity")).and_then(|v| v.as_str());
                            is_leader = current == Some(holder_id.as_str());
                            println!("[election] holderIdentity current={:?} is_leader={}", current, is_leader);
                        }
                    } else {
                        // Try to read for debugging
                        if let Ok(text) = resp.text().await {
                            println!("[election] renew body={} ", text);
                        }
                    }
                }
                Err(e) => println!("[election] renew error: {}", e),
            }

            if is_leader {
                println!("[leader] holding lease; pinging peers");
                for ep in &peers_for_task {
                    let uri = format!("http://{}", ep);
                    match Channel::from_shared(uri.clone()) {
                        Ok(builder) => match builder.connect().await {
                            Ok(ch) => {
                                let mut cli = KvClient::new(ch);
                                let req = GetRequest { key: b"ping".to_vec() };
                                match cli.get(tonic::Request::new(req)).await {
                                    Ok(_) => println!("[leader] ping {} ok", ep),
                                    Err(e) => println!("[leader] ping {} error: {}", ep, e),
                                }
                            }
                            Err(e) => println!("[leader] connect error to {}: {}", ep, e),
                        },
                        Err(e) => println!("[leader] bad uri {}: {}", ep, e),
                    }
                }
            } else {
                println!("[follower] not leader");
            }

            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
    tonic::transport::Server::builder()
        .add_service(KvServer::new(svc))
        .serve(addr)
        .await?;
    Ok(())
}
