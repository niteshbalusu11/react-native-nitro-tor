use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use anyhow::anyhow;
use logger::Logger;
use once_cell::sync::OnceCell;
use serde::Serialize;
use tor::control::TorControlEvent;
use tor::http_client::{make_http_request_async, HttpMethod, HttpRequestParams};
use tor::status::{BootstrapStatus, NetworkLiveness};
use tor::{
    ensure_runtime, onion_address_for_secret_key, OwnedTorService, TorErrors, TorHiddenService,
    TorHiddenServiceParam, TorService, TorServiceParam, TorStatusObserver,
};

pub type NativeResult<T> = Result<T, anyhow::Error>;
type StatusEmitter = Arc<dyn Fn(String) + Send + Sync + 'static>;
type ManagedTorService = Arc<Mutex<Box<dyn TorServiceAdapter>>>;

trait TorServiceAdapter: Send {
    fn control_address(&self) -> &str;
    fn get_connectivity(&self) -> Result<(NetworkLiveness, bool), TorErrors>;
    fn is_daemon_finished(&self) -> bool;
    fn request_new_identity(&self) -> Result<(), TorErrors>;
    fn create_hidden_service(
        &mut self,
        param: TorHiddenServiceParam,
    ) -> Result<TorHiddenService, TorErrors>;
    fn delete_hidden_service(&mut self, onion: String) -> Result<(), TorErrors>;
    fn shutdown(&mut self) -> Result<(), TorErrors>;
}

impl TorServiceAdapter for OwnedTorService {
    fn control_address(&self) -> &str {
        &self.control_port
    }

    fn get_connectivity(&self) -> Result<(NetworkLiveness, bool), TorErrors> {
        OwnedTorService::get_connectivity(self)
    }

    fn is_daemon_finished(&self) -> bool {
        OwnedTorService::is_daemon_finished(self)
    }

    fn request_new_identity(&self) -> Result<(), TorErrors> {
        OwnedTorService::request_new_identity(self)
    }

    fn create_hidden_service(
        &mut self,
        param: TorHiddenServiceParam,
    ) -> Result<TorHiddenService, TorErrors> {
        OwnedTorService::create_hidden_service(self, param)
    }

    fn delete_hidden_service(&mut self, onion: String) -> Result<(), TorErrors> {
        OwnedTorService::delete_hidden_service(self, onion)
    }

    fn shutdown(&mut self) -> Result<(), TorErrors> {
        OwnedTorService::shutdown(self)
    }
}

static MANAGER: OnceCell<TorManager> = OnceCell::new();
static LOGGER: OnceCell<Logger> = OnceCell::new();

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TorConfig {
    pub data_directory: String,
    pub socks_port: u16,
    pub bootstrap_timeout_ms: u64,
}

#[derive(Clone, Debug)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub headers_json: String,
    pub body: Option<String>,
    pub timeout_ms: u64,
    pub allow_invalid_certificates: bool,
}

#[derive(Clone, Debug)]
pub struct HiddenServiceOptions {
    pub virtual_port: u16,
    pub target_port: u16,
    pub private_key: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenService {
    pub onion_address: String,
    pub private_key: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TorErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum NetworkState {
    Up,
    Down,
    Unknown,
}

impl From<NetworkLiveness> for NetworkState {
    fn from(value: NetworkLiveness) -> Self {
        match value {
            NetworkLiveness::Up => Self::Up,
            NetworkLiveness::Down => Self::Down,
            NetworkLiveness::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Connectivity {
    network: NetworkState,
    circuit_established: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum TorStatus {
    Stopped,
    Starting {
        bootstrap: BootstrapStatus,
    },
    Running {
        #[serde(rename = "socksAddress")]
        socks_address: String,
        #[serde(rename = "controlAddress")]
        control_address: String,
        connectivity: Connectivity,
    },
    Stopping,
    Failed {
        error: TorErrorPayload,
    },
}

struct ManagerInner {
    status: TorStatus,
    pending_statuses: VecDeque<TorStatus>,
    config: Option<TorConfig>,
    service: Option<ManagedTorService>,
    start_in_progress: bool,
    cancellation: Arc<AtomicBool>,
    hidden_services: HashSet<String>,
}

impl ManagerInner {
    fn set_status(&mut self, status: TorStatus) -> TorStatus {
        self.status = status;
        self.queue_current_status()
    }

    fn queue_current_status(&mut self) -> TorStatus {
        let status = self.status.clone();
        self.pending_statuses.push_back(status.clone());
        status
    }
}

struct TorManager {
    inner: Mutex<ManagerInner>,
    state_changed: Condvar,
    emitter: Mutex<Option<StatusEmitter>>,
    publication: Mutex<()>,
}

impl TorManager {
    fn new() -> Self {
        Self {
            inner: Mutex::new(ManagerInner {
                status: TorStatus::Stopped,
                pending_statuses: VecDeque::new(),
                config: None,
                service: None,
                start_in_progress: false,
                cancellation: Arc::new(AtomicBool::new(false)),
                hidden_services: HashSet::new(),
            }),
            state_changed: Condvar::new(),
            emitter: Mutex::new(None),
            publication: Mutex::new(()),
        }
    }

    fn register_emitter(&self, emitter: StatusEmitter) {
        *self.emitter.lock().unwrap() = Some(emitter);
    }

    fn publish(&self, _status: &TorStatus) {
        let _publication = self.publication.lock().unwrap();
        loop {
            let status = self.inner.lock().unwrap().pending_statuses.pop_front();
            let Some(status) = status else {
                break;
            };
            let Ok(payload) = serde_json::to_string(&status) else {
                continue;
            };
            if let Some(emitter) = self.emitter.lock().unwrap().clone() {
                emitter(payload);
            }
        }
    }

    fn handle_control_event(&self, event: TorControlEvent) {
        let updated = {
            let mut inner = self.inner.lock().unwrap();
            let daemon_failure = if let TorControlEvent::ControlConnectionFailed(message) = &event {
                if matches!(inner.status, TorStatus::Running { .. })
                    && inner
                        .service
                        .as_ref()
                        .is_some_and(|service| service.lock().unwrap().is_daemon_finished())
                {
                    Some(message.clone())
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(message) = daemon_failure {
                inner.cancellation.store(true, Ordering::SeqCst);
                inner.hidden_services.clear();
                Some(inner.set_status(TorStatus::Failed {
                    error: TorErrorPayload {
                        code: "CONTROL_CONNECTION_FAILED".to_string(),
                        message,
                    },
                }))
            } else {
                let changed = match (&mut inner.status, event) {
                    (TorStatus::Starting { bootstrap }, TorControlEvent::Bootstrap(next)) => {
                        *bootstrap = next;
                        true
                    }
                    (
                        TorStatus::Running { connectivity, .. },
                        TorControlEvent::NetworkLiveness(network),
                    ) => {
                        connectivity.network = network.into();
                        true
                    }
                    (
                        TorStatus::Running { connectivity, .. },
                        TorControlEvent::CircuitEstablished(established),
                    ) => {
                        connectivity.circuit_established = established;
                        true
                    }
                    (
                        TorStatus::Running { connectivity, .. },
                        TorControlEvent::ControlConnectionFailed(_),
                    ) => {
                        let changed = connectivity.network != NetworkState::Unknown
                            || connectivity.circuit_established;
                        connectivity.network = NetworkState::Unknown;
                        connectivity.circuit_established = false;
                        changed
                    }
                    _ => false,
                };
                if changed {
                    Some(inner.queue_current_status())
                } else {
                    None
                }
            }
        };
        if let Some(status) = updated {
            self.publish(&status);
        }
    }
}

fn manager() -> &'static TorManager {
    LOGGER.get_or_init(Logger::new);
    let _ = ensure_runtime();
    MANAGER.get_or_init(TorManager::new)
}

pub fn register_status_emitter(emitter: StatusEmitter) {
    manager().register_emitter(emitter);
}

fn initial_bootstrap_status() -> BootstrapStatus {
    BootstrapStatus {
        progress: 0,
        tag: "starting".to_string(),
        summary: "Starting Tor".to_string(),
        warning: None,
    }
}

fn native_error(code: &str, message: impl Into<String>) -> anyhow::Error {
    let payload = TorErrorPayload {
        code: code.to_string(),
        message: message.into(),
    };
    anyhow!(
        "TOR_ERROR:{}",
        serde_json::to_string(&payload).unwrap_or_else(|_| {
            r#"{"code":"INTERNAL_ERROR","message":"Unable to serialize error"}"#.to_string()
        })
    )
}

fn start_error(error: TorErrors) -> (TorErrorPayload, anyhow::Error) {
    let (code, message) = match error {
        TorErrors::StartCancelled => ("TOR_STOPPED", "Tor stopped during startup".to_string()),
        TorErrors::BootstrapTimeout => (
            "BOOTSTRAP_TIMEOUT",
            "Timed out waiting for Tor to bootstrap".to_string(),
        ),
        error => ("TOR_START_FAILED", error.to_string()),
    };
    let payload = TorErrorPayload {
        code: code.to_string(),
        message: message.clone(),
    };
    (payload, native_error(code, message))
}

pub fn validate_config(
    data_directory: String,
    socks_port: f64,
    bootstrap_timeout_ms: f64,
) -> NativeResult<TorConfig> {
    if data_directory.trim().is_empty() {
        return Err(native_error(
            "INVALID_CONFIG",
            "dataDirectory must not be empty",
        ));
    }
    let socks_port = validate_port(socks_port, "socksPort", "INVALID_CONFIG")?;
    if !bootstrap_timeout_ms.is_finite()
        || bootstrap_timeout_ms.fract() != 0.0
        || bootstrap_timeout_ms <= 0.0
        || bootstrap_timeout_ms > u64::MAX as f64
    {
        return Err(native_error(
            "INVALID_CONFIG",
            "bootstrapTimeoutMs must be a positive integer",
        ));
    }
    Ok(TorConfig {
        data_directory,
        socks_port,
        bootstrap_timeout_ms: bootstrap_timeout_ms as u64,
    })
}

pub fn validate_port(value: f64, field: &str, code: &str) -> NativeResult<u16> {
    if !value.is_finite() || value.fract() != 0.0 || !(1.0..=65535.0).contains(&value) {
        return Err(native_error(
            code,
            format!("{field} must be an integer from 1 through 65535"),
        ));
    }
    Ok(value as u16)
}

pub fn validate_timeout(value: f64, field: &str, code: &str) -> NativeResult<u64> {
    if !value.is_finite() || value.fract() != 0.0 || value <= 0.0 || value > u64::MAX as f64 {
        return Err(native_error(
            code,
            format!("{field} must be a positive integer"),
        ));
    }
    Ok(value as u64)
}

pub fn start(config: TorConfig) -> NativeResult<String> {
    let launch_config = config.clone();
    start_with(manager(), config, move |observer, cancellation| {
        TorService::new(TorServiceParam {
            socks_port: Some(launch_config.socks_port),
            data_dir: launch_config.data_directory.clone(),
            bootstrap_timeout_ms: Some(launch_config.bootstrap_timeout_ms),
        })
        .and_then(|service| service.into_owned_node_with_observer(Some(observer), cancellation))
        .map(|service| Box::new(service) as Box<dyn TorServiceAdapter>)
    })
}

fn start_with<F>(manager: &'static TorManager, config: TorConfig, launch: F) -> NativeResult<String>
where
    F: FnOnce(TorStatusObserver, Arc<AtomicBool>) -> Result<Box<dyn TorServiceAdapter>, TorErrors>,
{
    {
        let mut inner = manager.inner.lock().unwrap();
        loop {
            match inner.status {
                TorStatus::Running { .. } => {
                    if inner.config.as_ref() != Some(&config) {
                        return Err(native_error(
                            "CONFIG_CONFLICT",
                            "Tor is already running with a different configuration",
                        ));
                    }
                    return serialize_status(&inner.status);
                }
                TorStatus::Starting { .. } => {
                    if inner.config.as_ref() != Some(&config) {
                        return Err(native_error(
                            "CONFIG_CONFLICT",
                            "Tor is already starting with a different configuration",
                        ));
                    }
                    while inner.start_in_progress {
                        inner = manager.state_changed.wait(inner).unwrap();
                    }
                    return match &inner.status {
                        TorStatus::Running { .. } => serialize_status(&inner.status),
                        TorStatus::Failed { error } => {
                            Err(native_error(&error.code, error.message.clone()))
                        }
                        TorStatus::Stopped => {
                            Err(native_error("TOR_STOPPED", "Tor stopped during startup"))
                        }
                        _ => Err(native_error(
                            "INTERNAL_ERROR",
                            "Tor startup completed in an invalid state",
                        )),
                    };
                }
                TorStatus::Stopping => {
                    inner = manager.state_changed.wait(inner).unwrap();
                }
                TorStatus::Failed { .. } if inner.service.is_some() => {
                    drop(inner);
                    dispose_retained_failed_service(manager)?;
                    inner = manager.inner.lock().unwrap();
                }
                TorStatus::Stopped | TorStatus::Failed { .. } => {
                    inner.config = Some(config.clone());
                    inner.cancellation = Arc::new(AtomicBool::new(false));
                    inner.start_in_progress = true;
                    inner.hidden_services.clear();
                    inner.set_status(TorStatus::Starting {
                        bootstrap: initial_bootstrap_status(),
                    });
                    let status = inner.status.clone();
                    drop(inner);
                    manager.publish(&status);
                    break;
                }
            }
        }
    }

    let cancellation = manager.inner.lock().unwrap().cancellation.clone();
    let observer = Arc::new(move |event| manager.handle_control_event(event));
    let service = launch(observer, cancellation.clone());

    match service {
        Ok(mut service) => {
            let connectivity = service
                .get_connectivity()
                .unwrap_or((NetworkLiveness::Unknown, false));
            if cancellation.load(Ordering::SeqCst) {
                let _ = service.shutdown();
                let status = {
                    let mut inner = manager.inner.lock().unwrap();
                    inner.start_in_progress = false;
                    inner.config = None;
                    inner.set_status(TorStatus::Stopped);
                    manager.state_changed.notify_all();
                    inner.status.clone()
                };
                manager.publish(&status);
                return Err(native_error("TOR_STOPPED", "Tor stopped during startup"));
            }

            let control_address = service.control_address().trim().to_string();
            let service = Arc::new(Mutex::new(service));
            let status = {
                let mut inner = manager.inner.lock().unwrap();
                inner.service = Some(service);
                inner.start_in_progress = false;
                inner.set_status(TorStatus::Running {
                    socks_address: format!("127.0.0.1:{}", config.socks_port),
                    control_address,
                    connectivity: Connectivity {
                        network: connectivity.0.into(),
                        circuit_established: connectivity.1,
                    },
                });
                manager.state_changed.notify_all();
                inner.status.clone()
            };
            manager.publish(&status);
            serialize_status(&status)
        }
        Err(error) => {
            let (payload, native_error) = start_error(error);
            let status = {
                let mut inner = manager.inner.lock().unwrap();
                inner.start_in_progress = false;
                if payload.code == "TOR_STOPPED" || matches!(inner.status, TorStatus::Stopping) {
                    inner.config = None;
                    inner.set_status(TorStatus::Stopped);
                } else {
                    inner.set_status(TorStatus::Failed {
                        error: payload.clone(),
                    });
                }
                manager.state_changed.notify_all();
                inner.status.clone()
            };
            manager.publish(&status);
            Err(native_error)
        }
    }
}

fn dispose_retained_failed_service(manager: &TorManager) -> NativeResult<()> {
    let Some((service, stopping)) = ({
        let mut inner = manager.inner.lock().unwrap();
        if !matches!(inner.status, TorStatus::Failed { .. }) {
            None
        } else {
            inner.service.take().map(|service| {
                inner.cancellation.store(true, Ordering::SeqCst);
                inner.set_status(TorStatus::Stopping);
                (service, inner.status.clone())
            })
        }
    }) else {
        return Ok(());
    };

    manager.publish(&stopping);
    let shutdown_result = service.lock().unwrap().shutdown();
    let stopped = {
        let mut inner = manager.inner.lock().unwrap();
        inner.config = None;
        inner.hidden_services.clear();
        inner.set_status(TorStatus::Stopped);
        manager.state_changed.notify_all();
        inner.status.clone()
    };
    manager.publish(&stopped);

    shutdown_result.map_err(|error| native_error("TOR_STOP_FAILED", error.to_string()))
}

pub fn stop() -> NativeResult<()> {
    stop_with(manager())
}

fn stop_with(manager: &'static TorManager) -> NativeResult<()> {
    let service = {
        let mut inner = manager.inner.lock().unwrap();
        match inner.status {
            TorStatus::Stopped => return Ok(()),
            TorStatus::Starting { .. } => {
                inner.cancellation.store(true, Ordering::SeqCst);
                inner.set_status(TorStatus::Stopping);
                let status = inner.status.clone();
                drop(inner);
                manager.publish(&status);
                inner = manager.inner.lock().unwrap();
                while inner.start_in_progress {
                    inner = manager.state_changed.wait(inner).unwrap();
                }
                if matches!(inner.status, TorStatus::Stopped) {
                    return Ok(());
                }
            }
            TorStatus::Stopping => {
                while matches!(inner.status, TorStatus::Stopping) {
                    inner = manager.state_changed.wait(inner).unwrap();
                }
                return Ok(());
            }
            TorStatus::Running { .. } | TorStatus::Failed { .. } => {}
        }
        inner.cancellation.store(true, Ordering::SeqCst);
        inner.set_status(TorStatus::Stopping);
        let status = inner.status.clone();
        let service = inner.service.take();
        drop(inner);
        manager.publish(&status);
        service
    };

    let shutdown_result = service
        .map(|service| service.lock().unwrap().shutdown())
        .transpose();
    let status = {
        let mut inner = manager.inner.lock().unwrap();
        inner.config = None;
        inner.hidden_services.clear();
        inner.set_status(TorStatus::Stopped);
        manager.state_changed.notify_all();
        inner.status.clone()
    };
    manager.publish(&status);
    shutdown_result
        .map_err(|error| native_error("TOR_STOP_FAILED", error.to_string()))
        .map(|_| ())
}

pub fn get_status() -> NativeResult<String> {
    get_status_with(manager())
}

enum ConnectivityObservation {
    Available(NetworkLiveness, bool),
    Unavailable,
    DaemonExited,
}

fn get_status_with(manager: &'static TorManager) -> NativeResult<String> {
    let service = {
        let inner = manager.inner.lock().unwrap();
        if matches!(inner.status, TorStatus::Running { .. }) {
            inner.service.clone()
        } else {
            None
        }
    };
    let observation = service.as_ref().map(|service| {
        let service = service.lock().unwrap();
        if service.is_daemon_finished() {
            ConnectivityObservation::DaemonExited
        } else {
            match service.get_connectivity() {
                Ok((network, circuit_established)) => {
                    ConnectivityObservation::Available(network, circuit_established)
                }
                Err(_) => ConnectivityObservation::Unavailable,
            }
        }
    });
    let (status, changed) = {
        let mut inner = manager.inner.lock().unwrap();
        let mut changed = false;
        let mut queued = false;
        let same_service = service.as_ref().is_some_and(|observed| {
            inner
                .service
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(observed, current))
        });
        if same_service && matches!(inner.status, TorStatus::Running { .. }) {
            match observation {
                Some(ConnectivityObservation::Available(network, circuit_established)) => {
                    if let TorStatus::Running { connectivity, .. } = &mut inner.status {
                        let network = network.into();
                        if connectivity.network != network
                            || connectivity.circuit_established != circuit_established
                        {
                            connectivity.network = network.into();
                            connectivity.circuit_established = circuit_established;
                            changed = true;
                        }
                    }
                }
                Some(ConnectivityObservation::Unavailable) => {
                    if let TorStatus::Running { connectivity, .. } = &mut inner.status {
                        if connectivity.network != NetworkState::Unknown
                            || connectivity.circuit_established
                        {
                            connectivity.network = NetworkState::Unknown;
                            connectivity.circuit_established = false;
                            changed = true;
                        }
                    }
                }
                Some(ConnectivityObservation::DaemonExited) => {
                    inner.cancellation.store(true, Ordering::SeqCst);
                    inner.hidden_services.clear();
                    inner.set_status(TorStatus::Failed {
                        error: TorErrorPayload {
                            code: "CONTROL_CONNECTION_FAILED".to_string(),
                            message: "Tor daemon exited unexpectedly".to_string(),
                        },
                    });
                    changed = true;
                    queued = true;
                }
                None => {}
            }
        }
        if changed && !queued {
            inner.queue_current_status();
        }
        (inner.status.clone(), changed)
    };
    if changed {
        manager.publish(&status);
    }
    serialize_status(&status)
}

pub fn request_new_identity() -> NativeResult<()> {
    let manager = manager();
    let service = {
        let inner = manager.inner.lock().unwrap();
        if !matches!(inner.status, TorStatus::Running { .. }) {
            return Err(native_error(
                "NOT_RUNNING",
                "Tor must be running to request a new identity",
            ));
        }
        inner
            .service
            .as_ref()
            .cloned()
            .ok_or_else(|| native_error("NOT_RUNNING", "Tor is not running"))?
    };
    let result = service
        .lock()
        .unwrap()
        .request_new_identity()
        .map_err(|error| native_error("NEW_IDENTITY_FAILED", error.to_string()));
    result
}

pub fn http_request(request: HttpRequest) -> NativeResult<String> {
    if request.url.trim().is_empty() {
        return Err(native_error("INVALID_REQUEST", "url must not be empty"));
    }
    if request.timeout_ms == 0 {
        return Err(native_error(
            "INVALID_REQUEST",
            "timeoutMs must be greater than zero",
        ));
    }
    let method = parse_http_method(&request.method)?;
    let headers: Option<HashMap<String, String>> =
        if request.headers_json.is_empty() {
            None
        } else {
            Some(serde_json::from_str(&request.headers_json).map_err(|_| {
                native_error("INVALID_REQUEST", "headers must contain string values")
            })?)
        };
    let (socks_proxy, cancellation) = {
        let inner = manager().inner.lock().unwrap();
        let socks_port = match &inner.status {
            TorStatus::Running { .. } => inner.config.as_ref().map(|config| config.socks_port),
            _ => None,
        }
        .ok_or_else(|| native_error("NOT_RUNNING", "Tor is not running"))?;
        (
            format!("127.0.0.1:{socks_port}"),
            inner.cancellation.clone(),
        )
    };

    let params = HttpRequestParams {
        url: request.url,
        method,
        headers,
        body: request.body,
        timeout_ms: Some(request.timeout_ms),
        trust_invalid_certs: Some(request.allow_invalid_certificates),
    };
    let result = ensure_runtime().block_on(async {
        tokio::select! {
            response = make_http_request_async(params, socks_proxy) => response,
            _ = wait_for_cancellation(cancellation) => Err(TorErrors::StartCancelled),
        }
    });
    match result {
        Ok(response) => serde_json::to_string(&response)
            .map_err(|error| native_error("INTERNAL_ERROR", error.to_string())),
        Err(TorErrors::StartCancelled) => Err(native_error(
            "TOR_STOPPED",
            "Tor stopped before the HTTP request completed",
        )),
        Err(TorErrors::HttpRequestError(error)) if error.is_timeout() => {
            Err(native_error("HTTP_TIMEOUT", error.to_string()))
        }
        Err(error) => Err(native_error("HTTP_TRANSPORT_ERROR", error.to_string())),
    }
}

async fn wait_for_cancellation(cancellation: Arc<AtomicBool>) {
    while !cancellation.load(Ordering::SeqCst) {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

fn parse_http_method(method: &str) -> NativeResult<HttpMethod> {
    match method.trim().to_ascii_uppercase().as_str() {
        "GET" => Ok(HttpMethod::GET),
        "POST" => Ok(HttpMethod::POST),
        "PUT" => Ok(HttpMethod::PUT),
        "DELETE" => Ok(HttpMethod::DELETE),
        "HEAD" => Ok(HttpMethod::HEAD),
        "OPTIONS" => Ok(HttpMethod::OPTIONS),
        _ => Err(native_error(
            "INVALID_REQUEST",
            format!("Unsupported HTTP method: {method}"),
        )),
    }
}

pub fn create_hidden_service(options: HiddenServiceOptions) -> NativeResult<HiddenService> {
    let private_key =
        match options.private_key.len() {
            0 => None,
            64 => {
                Some(options.private_key.as_slice().try_into().map_err(|_| {
                    native_error("INVALID_PRIVATE_KEY", "privateKey must be 64 bytes")
                })?)
            }
            _ => {
                return Err(native_error(
                    "INVALID_PRIVATE_KEY",
                    "privateKey must be exactly 64 bytes",
                ));
            }
        };

    let manager = manager();
    let mut inner = manager.inner.lock().unwrap();
    if !matches!(inner.status, TorStatus::Running { .. }) {
        return Err(native_error(
            "NOT_RUNNING",
            "Tor must be running to create a hidden service",
        ));
    }
    if let Some(private_key) = private_key {
        let onion_address = onion_address_for_secret_key(private_key);
        if inner.hidden_services.contains(&onion_address) {
            return Err(native_error(
                "HIDDEN_SERVICE_EXISTS",
                "The hidden service is already active",
            ));
        }
    }
    let service = inner
        .service
        .as_ref()
        .ok_or_else(|| native_error("NOT_RUNNING", "Tor is not running"))?;
    let result = service
        .lock()
        .unwrap()
        .create_hidden_service(TorHiddenServiceParam {
            to_port: options.target_port,
            hs_port: options.virtual_port,
            secret_key: private_key,
        })
        .map_err(|error| native_error("HIDDEN_SERVICE_ERROR", error.to_string()))?;
    let onion_address = result
        .onion_url
        .to_string()
        .rsplit_once(':')
        .map(|(address, _)| address.to_string())
        .unwrap_or_else(|| result.onion_url.to_string());
    if !inner.hidden_services.insert(onion_address.clone()) {
        return Err(native_error(
            "HIDDEN_SERVICE_EXISTS",
            "The hidden service is already active",
        ));
    }
    Ok(HiddenService {
        onion_address,
        private_key: result.secret_key.to_vec(),
    })
}

pub fn remove_hidden_service(onion_address: String) -> NativeResult<()> {
    let manager = manager();
    let mut inner = manager.inner.lock().unwrap();
    let onion_address = onion_address.trim().to_string();
    if !inner.hidden_services.remove(&onion_address) {
        return Ok(());
    }
    if !matches!(inner.status, TorStatus::Running { .. }) {
        inner.hidden_services.insert(onion_address);
        return Err(native_error(
            "NOT_RUNNING",
            "Tor must be running to remove a hidden service",
        ));
    }
    let result = inner
        .service
        .as_ref()
        .ok_or_else(|| native_error("NOT_RUNNING", "Tor is not running"))?
        .lock()
        .unwrap()
        .delete_hidden_service(onion_address.clone());
    if let Err(error) = result {
        inner.hidden_services.insert(onion_address);
        return Err(native_error("HIDDEN_SERVICE_ERROR", error.to_string()));
    }
    Ok(())
}

fn serialize_status(status: &TorStatus) -> NativeResult<String> {
    serde_json::to_string(status).map_err(|error| native_error("INTERNAL_ERROR", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    struct FakeTorService {
        control_address: String,
        connectivity_gate: Option<Arc<(Mutex<(usize, bool)>, Condvar)>>,
        daemon_finished: bool,
    }

    impl TorServiceAdapter for FakeTorService {
        fn control_address(&self) -> &str {
            &self.control_address
        }

        fn get_connectivity(&self) -> Result<(NetworkLiveness, bool), TorErrors> {
            if let Some((lock, changed)) = self.connectivity_gate.as_deref() {
                let mut state = lock.lock().unwrap();
                state.0 += 1;
                changed.notify_all();
                while state.0 > 1 && !state.1 {
                    state = changed.wait(state).unwrap();
                }
            }
            Ok((NetworkLiveness::Up, true))
        }

        fn is_daemon_finished(&self) -> bool {
            self.daemon_finished
        }

        fn request_new_identity(&self) -> Result<(), TorErrors> {
            Ok(())
        }

        fn create_hidden_service(
            &mut self,
            _param: TorHiddenServiceParam,
        ) -> Result<tor::TorHiddenService, TorErrors> {
            Err(TorErrors::BootStrapError("unused fake operation".into()))
        }

        fn delete_hidden_service(&mut self, _onion: String) -> Result<(), TorErrors> {
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), TorErrors> {
            Ok(())
        }
    }

    fn fake_service() -> Box<dyn TorServiceAdapter> {
        Box::new(FakeTorService {
            control_address: "127.0.0.1:29051".to_string(),
            connectivity_gate: None,
            daemon_finished: false,
        })
    }

    #[test]
    fn validates_native_config_numbers() {
        let config = validate_config("/tmp/tor".to_string(), 19050.0, 45_000.0).unwrap();
        assert_eq!(config.socks_port, 19050);
        assert!(validate_config("".to_string(), 19050.0, 45_000.0).is_err());
        assert!(validate_config("/tmp/tor".to_string(), 0.0, 45_000.0).is_err());
        assert!(validate_config("/tmp/tor".to_string(), 19050.5, 45_000.0).is_err());
        assert!(validate_config("/tmp/tor".to_string(), 19050.0, 0.0).is_err());
    }

    #[test]
    fn serializes_public_status_shape() {
        let status = TorStatus::Running {
            socks_address: "127.0.0.1:19050".to_string(),
            control_address: "127.0.0.1:19051".to_string(),
            connectivity: Connectivity {
                network: NetworkState::Up,
                circuit_established: true,
            },
        };
        assert_eq!(
            serialize_status(&status).unwrap(),
            r#"{"state":"running","socksAddress":"127.0.0.1:19050","controlAddress":"127.0.0.1:19051","connectivity":{"network":"up","circuitEstablished":true}}"#
        );
    }

    #[test]
    fn accepts_only_supported_http_methods() {
        assert!(matches!(parse_http_method("get"), Ok(HttpMethod::GET)));
        assert!(parse_http_method("PATCH").is_err());
    }

    #[test]
    fn publishes_queued_statuses_in_transition_order() {
        let manager = TorManager::new();
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let captured = emitted.clone();
        manager.register_emitter(Arc::new(move |status| {
            captured.lock().unwrap().push(status);
        }));

        let latest = {
            let mut inner = manager.inner.lock().unwrap();
            inner.set_status(TorStatus::Starting {
                bootstrap: initial_bootstrap_status(),
            });
            inner.set_status(TorStatus::Stopping)
        };
        manager.publish(&latest);

        let emitted = emitted.lock().unwrap();
        assert_eq!(emitted.len(), 2);
        assert!(emitted[0].contains(r#""state":"starting""#));
        assert_eq!(emitted[1], r#"{"state":"stopping"}"#);
    }

    #[test]
    fn control_monitor_failure_clears_stale_connectivity() {
        let manager = TorManager::new();
        manager.inner.lock().unwrap().status = TorStatus::Running {
            socks_address: "127.0.0.1:19050".to_string(),
            control_address: "127.0.0.1:19051".to_string(),
            connectivity: Connectivity {
                network: NetworkState::Up,
                circuit_established: true,
            },
        };

        manager.handle_control_event(TorControlEvent::ControlConnectionFailed(
            "disconnected".to_string(),
        ));

        assert!(matches!(
            &manager.inner.lock().unwrap().status,
            TorStatus::Running {
                connectivity: Connectivity {
                    network: NetworkState::Unknown,
                    circuit_established: false,
                },
                ..
            }
        ));
    }

    #[test]
    fn identical_concurrent_starts_share_one_attempt() {
        let manager: &'static TorManager = Box::leak(Box::new(TorManager::new()));
        let config = TorConfig {
            data_directory: "/tmp/react-native-nitro-tor-shared-start".to_string(),
            socks_port: 29050,
            bootstrap_timeout_ms: 45_000,
        };
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let (entered_tx, entered_rx) = mpsc::channel();
        let first_release = release.clone();
        let first_config = config.clone();
        let first = thread::spawn(move || {
            start_with(manager, first_config, move |_observer, _cancellation| {
                entered_tx.send(()).unwrap();
                let (lock, changed) = &*first_release;
                let mut released = lock.lock().unwrap();
                while !*released {
                    released = changed.wait(released).unwrap();
                }
                Ok(fake_service())
            })
        });
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        let second_config = config.clone();
        let second = thread::spawn(move || {
            start_with(manager, second_config, |_observer, _cancellation| {
                panic!("a matching concurrent start must not launch another daemon")
            })
        });
        thread::sleep(Duration::from_millis(50));
        assert!(!second.is_finished());

        let (lock, changed) = &*release;
        *lock.lock().unwrap() = true;
        changed.notify_all();

        let first_status = first.join().unwrap().unwrap();
        let second_status = second.join().unwrap().unwrap();
        assert_eq!(first_status, second_status);
        assert!(first_status.contains(r#""state":"running""#));
        stop_with(manager).unwrap();
    }

    #[test]
    fn conflicting_start_rejects_and_stop_cancels_the_active_attempt() {
        let manager: &'static TorManager = Box::leak(Box::new(TorManager::new()));
        let config = TorConfig {
            data_directory: "/tmp/react-native-nitro-tor-cancel-start".to_string(),
            socks_port: 29350,
            bootstrap_timeout_ms: 45_000,
        };
        let (entered_tx, entered_rx) = mpsc::channel();
        let first_config = config.clone();
        let first = thread::spawn(move || {
            start_with(manager, first_config, move |_observer, cancellation| {
                entered_tx.send(()).unwrap();
                while !cancellation.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(TorErrors::StartCancelled)
            })
        });
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();

        let conflict = start_with(
            manager,
            TorConfig {
                socks_port: 29351,
                ..config
            },
            |_observer, _cancellation| {
                panic!("a conflicting start must reject before launching a daemon")
            },
        )
        .unwrap_err();
        assert!(conflict.to_string().contains("CONFIG_CONFLICT"));

        stop_with(manager).unwrap();
        let start_error = first.join().unwrap().unwrap_err();
        assert!(start_error.to_string().contains("TOR_STOPPED"));
        assert_eq!(get_status_with(manager).unwrap(), r#"{"state":"stopped"}"#);
    }

    #[test]
    fn status_probe_does_not_block_the_stopping_transition() {
        let manager: &'static TorManager = Box::leak(Box::new(TorManager::new()));
        let config = TorConfig {
            data_directory: "/tmp/react-native-nitro-tor-status-probe".to_string(),
            socks_port: 29150,
            bootstrap_timeout_ms: 45_000,
        };
        let gate = Arc::new((Mutex::new((0, false)), Condvar::new()));
        let service_gate = gate.clone();
        start_with(manager, config, move |_observer, _cancellation| {
            Ok(Box::new(FakeTorService {
                control_address: "127.0.0.1:29151".to_string(),
                connectivity_gate: Some(service_gate),
                daemon_finished: false,
            }))
        })
        .unwrap();

        let (status_tx, status_rx) = mpsc::channel();
        manager.register_emitter(Arc::new(move |status| {
            status_tx.send(status).unwrap();
        }));
        let status_thread = thread::spawn(move || get_status_with(manager));
        let (lock, changed) = &*gate;
        let mut state = lock.lock().unwrap();
        while state.0 < 2 {
            state = changed.wait(state).unwrap();
        }
        drop(state);

        let stop_thread = thread::spawn(move || stop_with(manager));
        let stopping = status_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(stopping, r#"{"state":"stopping"}"#);

        let mut state = lock.lock().unwrap();
        state.1 = true;
        changed.notify_all();
        drop(state);
        status_thread.join().unwrap().unwrap();
        stop_thread.join().unwrap().unwrap();
    }

    #[test]
    fn control_failure_marks_an_exited_daemon_failed() {
        let manager: &'static TorManager = Box::leak(Box::new(TorManager::new()));
        let config = TorConfig {
            data_directory: "/tmp/react-native-nitro-tor-exited-daemon".to_string(),
            socks_port: 29250,
            bootstrap_timeout_ms: 45_000,
        };
        start_with(manager, config, |_observer, _cancellation| {
            Ok(Box::new(FakeTorService {
                control_address: "127.0.0.1:29251".to_string(),
                connectivity_gate: None,
                daemon_finished: true,
            }))
        })
        .unwrap();

        manager.handle_control_event(TorControlEvent::ControlConnectionFailed(
            "connection closed".to_string(),
        ));

        let inner = manager.inner.lock().unwrap();
        assert!(inner.cancellation.load(Ordering::SeqCst));
        assert!(matches!(
            &inner.status,
            TorStatus::Failed {
                error: TorErrorPayload { code, .. }
            } if code == "CONTROL_CONNECTION_FAILED"
        ));
    }
}
