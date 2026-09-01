pub mod control;
pub mod hidden_service;
pub mod http_client;
pub mod status;
pub mod tcp_stream;
use futures::Future;
use libtor::{Tor, TorAddress, TorFlag};
use logger::log::*;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::convert::{TryFrom, TryInto};
use std::fs;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::task::JoinError;
use tokio::time::{Duration, timeout};
use tokio_compat_02::FutureExt;
use torut::control::{AsyncEvent, AuthenticatedConn, ConnError, UnauthenticatedConn};
use torut::onion::TorSecretKeyV3;

use crate::control::{ControlConnection, TorControlEvent};
use crate::status::{
    NetworkLiveness, parse_bootstrap_status, parse_circuit_established, parse_network_liveness,
};

type F = Box<
    dyn Fn(AsyncEvent<'static>) -> Pin<Box<dyn Future<Output = Result<(), ConnError>>>>
        + Send
        + Sync,
>;
type G = AuthenticatedConn<TcpStream, F>;

// Replace lazy_static with once_cell for better initialization control
static RUNTIME: OnceCell<tokio::runtime::Runtime> = OnceCell::new();

pub fn ensure_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .max_blocking_threads(num_cpus::get() / 2)
            .thread_name_fn(|| {
                static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                format!("sifir-thread-pool-{}", id)
            })
            .on_thread_start(|| debug!("thread started on {} cpus", num_cpus::get()))
            .on_thread_stop(|| debug!("thread stopped"))
            .enable_all()
            .build()
            .unwrap()
    })
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
pub struct TorServiceParam {
    pub socks_port: Option<u16>,
    pub data_dir: String,
    pub bootstrap_timeout_ms: Option<u64>,
}

impl TorServiceParam {
    pub fn new(data_dir: &str, socks_port: u16, bootstap_timeout_ms: u64) -> TorServiceParam {
        TorServiceParam {
            data_dir: String::from(data_dir),
            socks_port: Some(socks_port),
            bootstrap_timeout_ms: Some(bootstap_timeout_ms),
        }
    }
}

pub struct TorService {
    socks_port: u16,
    control_port: String,
    bootstrap_timeout_ms: u64,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
}

pub struct OwnedTorService {
    pub socks_port: u16,
    pub control_port: String,
    _handle: Option<JoinHandle<Result<u8, libtor::Error>>>,
    _ctl: RefCell<Option<G>>,
    event_handle: Option<tokio::task::JoinHandle<()>>,
}

pub type TorStatusObserver = Arc<dyn Fn(TorControlEvent) + Send + Sync + 'static>;

#[repr(C)]
pub struct TorHiddenServiceParam {
    pub to_port: u16,
    pub hs_port: u16,
    pub secret_key: Option<[u8; 64]>,
}

#[derive(Debug)]
pub struct TorHiddenService {
    pub onion_url: TorAddress,
    pub secret_key: [u8; 64],
}

pub fn onion_address_for_secret_key(secret_key: [u8; 64]) -> String {
    TorSecretKeyV3::from(secret_key)
        .public()
        .get_onion_address()
        .to_string()
}

fn query_connectivity(
    control_port: &str,
    query_timeout: Duration,
) -> Result<(NetworkLiveness, bool), TorErrors> {
    let control_port = control_port.to_string();
    ensure_runtime().block_on(async move {
        match timeout(query_timeout, async move {
            let mut connection = ControlConnection::connect(&control_port).await?;
            let network = connection.get_info("network-liveness").await?;
            let circuit = connection.get_info("status/circuit-established").await?;
            Ok::<_, TorErrors>((
                parse_network_liveness(&network),
                parse_circuit_established(&circuit).unwrap_or(false),
            ))
        })
        .await
        {
            Ok(result) => result,
            Err(_) => Err(TorErrors::ControlTimeout),
        }
    })
}

fn normalize_onion_service_id(onion: &str) -> &str {
    onion
        .split(':')
        .next()
        .unwrap_or(onion)
        .trim_end_matches(".onion")
}
/// The Phases of a Boostraping node
/// From https://github.com/torproject/torspec/blob/master/proposals/137-bootstrap-phases.txt
#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
/// String describing the current bootstarp phase of the node
pub struct BootstrapPhase(String);

#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
/// Describes the BootstrapPhase the Tor daemon is in.
pub enum OwnedTorServiceBootstrapPhase {
    // Daemon is done Boostraping and is ready to use
    Done,
    // Still bootstraping or error
    Other(BootstrapPhase),
}
/// High level API for Torut's AuthenticatedConnection used internally by TorService to expose
/// note control functions to FFI and user
trait TorControlApi {
    // async fns in traits are a shit show
    fn wait_bootstrap(
        &mut self,
        timeout_ms: Option<u64>,
        observer: Option<TorStatusObserver>,
        cancelled: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, TorErrors>> + '_>>;
    fn get_status(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<OwnedTorServiceBootstrapPhase, TorErrors>> + '_>>;
}

#[derive(Error, Debug)]
pub enum TorErrors {
    #[error("Control connection error: {:?}",.0)]
    ControlConnectionError(#[from] ConnError),
    #[error("Error with Tor daemon:")]
    TorLibError(#[from] libtor::Error),
    #[error("Error Bootstraping:")]
    BootStrapError(String),
    #[error("Tor startup was cancelled")]
    StartCancelled,
    #[error("Timed out waiting for Tor to bootstrap")]
    BootstrapTimeout,
    #[error("Timed out waiting for the Tor control connection")]
    ControlTimeout,
    #[error("Error Io:")]
    IoError(#[from] io::Error),
    #[error("Error Threading:")]
    ThreadingError(#[from] JoinError),
    #[error("Error TcpStream:")]
    TcpStreamError(String),
    #[error("HTTP request failed: {0}")]
    HttpRequestError(#[from] reqwest::Error),
}

/// Convert Torservice Param into an Unauthentication TorService:
/// Instantiates the Tor service on a seperate thread, however does not take ownership
/// nor await it's completion of the BootstrapPhase
// TODO make timeout a param, but how can we kill backgroun without having access ?
impl TryFrom<TorServiceParam> for TorService {
    type Error = TorErrors;
    fn try_from(param: TorServiceParam) -> Result<Self, Self::Error> {
        let mut service = Tor::new();
        let socks_port = param.socks_port.unwrap_or(19051);
        let base_dir = format!("{}/sifir_sdk/tor", param.data_dir);
        let data_dir = format!("{}/data", base_dir);
        let cache_dir = format!("{}/cache", base_dir);
        let ctl_file_path = format!("{}/ctl.info", base_dir);
        let info_log_path = format!("{}/logs/sifir_tor_log.info", base_dir);
        let error_log_path = format!("{}/logs/sifir_tor_log.err", base_dir);
        // Create directories
        fs::create_dir_all(data_dir.clone())?;
        fs::create_dir_all(format!("{}/logs", base_dir))?;
        fs::create_dir_all(cache_dir.clone())?;
        // Setup logfiles
        // Create logfile if not existing to avoid issues with mobile
        // Vector Of Results -> Result of Vectors
        let logfiles_check: Result<Vec<_>, _> = vec![&info_log_path, &error_log_path]
            .iter()
            .map(|p| {
                fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .create_new(true)
                    .open(p)
            })
            .map(|fr| match fr {
                Ok(_) => Ok(()),
                Err(e) => match e.kind() {
                    std::io::ErrorKind::AlreadyExists => {
                        debug!("Log file already exists: {}", e);
                        Ok(())
                    }
                    _ => Err(TorErrors::IoError(e)),
                },
            })
            .collect();
        let _ = logfiles_check?;
        service
            .flag(TorFlag::DataDirectory(data_dir))
            // Note: Making data dir group readble breaks android
            //.flag(TorFlag::DataDirectoryGroupReadable(TorBool::True))
            .flag(TorFlag::CacheDirectory(cache_dir))
            //.flag(TorFlag::CacheDirectoryGroupReadable("1".into()))
            .flag(TorFlag::SocksPort(socks_port))
            .flag(TorFlag::ControlPortAuto)
            .flag(TorFlag::CookieAuthentication(libtor::TorBool::True))
            .flag(TorFlag::ControlPortWriteToFile(ctl_file_path.clone()))
            .flag(TorFlag::ControlPortFileGroupReadable(libtor::TorBool::True));
        // // Android logging to android
        // #[cfg(target_os = "android")]
        // {
        //     service
        //         .flag(TorFlag::AndroidIdentityTag("com.sifir.tor".into()))
        //         .flag(TorFlag::LogTo(
        //             libtor::LogLevel::Debug,
        //             libtor::LogDestination::Android,
        //         ));
        // }

        let handle = service.start_background();

        let mut is_ready = false;
        let mut control_port = String::new();
        let mut try_times = 0;
        // We wait for Tor to write the new config file otherwise we risk reading the old config and port.
        // Anything less than a second and iOS errors out
        // TODO Anyway to *know* when the new config has been written besides checking config file modifed after starting process?
        std::thread::sleep(std::time::Duration::from_millis(1000));
        while !is_ready {
            let contents = fs::read_to_string(ctl_file_path.clone());
            match contents {
                Ok(t) => {
                    if !t.contains("PORT=") {
                        return Err(TorErrors::BootStrapError(String::from("No port in config")));
                    };
                    let data: Vec<&str> = t.split("PORT=").collect();
                    control_port = data[1].into();
                    info!("success with config port {}!", control_port);
                    is_ready = true;
                }
                Err(_) => {
                    try_times += 1;
                    if try_times > 10 {
                        return Err(TorErrors::BootStrapError(String::from(
                            "Unable to read daemon control info",
                        )));
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(900));
        }

        Ok(TorService {
            socks_port,
            control_port,
            bootstrap_timeout_ms: param.bootstrap_timeout_ms.unwrap_or(45000),
            _handle: Some(handle),
        })
    }
}
/// Async handler injected into Torut to recieve Tor daemon async events
/// Right now does nothing but is needed for AuthenticatedConnection from Torut to function correctly
fn handler(_: AsyncEvent<'static>) -> Pin<Box<dyn Future<Output = Result<(), ConnError>>>> {
    Box::pin(async move { Ok(()) })
}

impl TorService {
    pub fn new(param: TorServiceParam) -> Result<Self, TorErrors> {
        param.try_into()
    }
    async fn get_control_auth_conn<F>(
        &self,
        handle: Option<F>,
    ) -> Result<AuthenticatedConn<TcpStream, F>, TorErrors> {
        let s = TcpStream::connect(self.control_port.trim()).await?;
        let mut utc = UnauthenticatedConn::new(s);
        // returns node info + cookie location
        let proto_info = utc
            .load_protocol_info()
            .await
            .map_err(TorErrors::ControlConnectionError)?;
        // loads cookie from loaded data and build auth info
        let auth = proto_info
            .make_auth_data()?
            .ok_or(TorErrors::BootStrapError(String::from(
                "Error making control auth data",
            )))?;
        utc.authenticate(&auth)
            .await
            .map_err(TorErrors::ControlConnectionError)?;
        // upgrade connection to authenticated
        let mut ac = utc.into_authenticated().await;
        if handle.is_some() {
            ac.set_async_event_handler(handle);
        }
        Ok(ac)
    }

    /// Converts TorService to OwnedTorService, consuming the TorService
    /// and returning an OwnedTorService which is fully bootstrapped and under our control
    /// (If we drop this object the Tor daemon will shut down)
    pub fn into_owned_node(self) -> Result<OwnedTorService, TorErrors> {
        self.into_owned_node_with_observer(None, Arc::new(AtomicBool::new(false)))
    }

    pub fn into_owned_node_with_observer(
        self,
        observer: Option<TorStatusObserver>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<OwnedTorService, TorErrors> {
        let bootstrap_timeout_ms = self.bootstrap_timeout_ms;
        let event_cancellation = cancelled.clone();
        let connection = ensure_runtime().block_on(
            async {
                let mut ac = self
                    .get_control_auth_conn(Some(Box::new(handler) as F))
                    .compat()
                    .await?;
                // Take ownership before bootstrap so dropping this connection stops Tor.
                ac.take_ownership()
                    .compat()
                    .await
                    .map_err(TorErrors::ControlConnectionError)?;
                ac.wait_bootstrap(Some(bootstrap_timeout_ms), observer.clone(), cancelled)
                    .await?;
                Ok(ac)
            }
            .compat(),
        );
        let socks_port = self.socks_port;
        let control_port = self.control_port;
        let handle = self._handle;

        match connection {
            Ok(ac) => {
                let mut service = OwnedTorService {
                    socks_port,
                    control_port,
                    _handle: handle,
                    _ctl: RefCell::new(Some(ac)),
                    event_handle: None,
                };
                service.start_event_monitor(observer, event_cancellation);
                Ok(service)
            }
            Err(error) => {
                if let Some(handle) = handle {
                    handle.join().map_err(|_| {
                        TorErrors::BootStrapError(
                            "Error joining Tor after failed startup".to_string(),
                        )
                    })??;
                }
                Err(error)
            }
        }
    }
}

impl TryFrom<TorServiceParam> for OwnedTorService {
    type Error = TorErrors;
    fn try_from(param: TorServiceParam) -> Result<Self, Self::Error> {
        let t: TorService = param.try_into()?;
        t.into_owned_node()
    }
}

/// Implementation when TorService has AuthenticatedConnection established
/// This is what the FFI and most external libs should be interacting with
impl OwnedTorService {
    pub fn new(param: TorServiceParam) -> Result<Self, TorErrors> {
        let owned_result: Result<OwnedTorService, TorErrors> = param.try_into();
        owned_result
    }
    pub fn create_hidden_service(
        &mut self,
        param: TorHiddenServiceParam,
    ) -> Result<TorHiddenService, TorErrors> {
        ensure_runtime().block_on(
            async {
                let mut _ctl = self._ctl.borrow_mut();
                let ctl = _ctl
                    .as_mut()
                    .ok_or(TorErrors::BootStrapError(String::from("Error mut lock")))?;

                let service_key = match param.secret_key {
                    Some(key) => key.into(),
                    _ => TorSecretKeyV3::generate(),
                };

                ctl.add_onion_v3(
                    &service_key,
                    false,
                    false,
                    false,
                    None,
                    &mut [(
                        param.hs_port,
                        SocketAddr::new(IpAddr::from(Ipv4Addr::new(127, 0, 0, 1)), param.to_port),
                    )]
                    .iter(),
                )
                .await
                .map_err(TorErrors::ControlConnectionError)?;

                info!("Hidden service created!");
                let onion_url = TorAddress::AddressPort(
                    service_key.public().get_onion_address().to_string(),
                    param.hs_port,
                );
                let secret_key = service_key.as_bytes();
                Ok(TorHiddenService {
                    onion_url,
                    secret_key,
                })
            }
            .compat(),
        )
    }
    pub fn delete_hidden_service(&mut self, onion: String) -> Result<(), TorErrors> {
        ensure_runtime().block_on(
            async {
                let mut _ctl = self._ctl.borrow_mut();
                let ctl = _ctl
                    .as_mut()
                    .ok_or(TorErrors::BootStrapError(String::from("Error mut lock")))?;

                ctl.del_onion(normalize_onion_service_id(&onion))
                    .await
                    .map_err(TorErrors::ControlConnectionError)?;

                info!("Hidden serviec deleted !");
                Ok(())
            }
            .compat(),
        )
    }

    /// Get the status of the Tor daemon we own
    /// OwnedTorServiceBootstrapPhase will either be Done or Other(String) containing the stage of
    /// the boostrap the node is a
    pub fn get_status(&self) -> Result<OwnedTorServiceBootstrapPhase, TorErrors> {
        ensure_runtime().block_on(
            async {
                let mut ctl = self._ctl.borrow_mut();
                let r = ctl
                    .as_mut()
                    .ok_or(TorErrors::BootStrapError("Unable to get mut".into()))?
                    .get_status()
                    .await?;
                Ok(r)
            }
            .compat(),
        )
    }

    pub fn get_connectivity(&self) -> Result<(NetworkLiveness, bool), TorErrors> {
        query_connectivity(&self.control_port, Duration::from_secs(2))
    }

    pub fn request_new_identity(&self) -> Result<(), TorErrors> {
        let control_port = self.control_port.clone();
        ensure_runtime().block_on(async move {
            let mut connection = ControlConnection::connect(&control_port).await?;
            connection.request_new_identity().await
        })
    }

    pub fn is_daemon_finished(&self) -> bool {
        self._handle
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
    }

    fn start_event_monitor(
        &mut self,
        observer: Option<TorStatusObserver>,
        cancelled: Arc<AtomicBool>,
    ) {
        let Some(observer) = observer else {
            return;
        };
        let control_port = self.control_port.clone();
        self.event_handle = Some(ensure_runtime().spawn(async move {
            let mut retry_delay = std::time::Duration::from_millis(250);
            while !cancelled.load(Ordering::SeqCst) {
                let mut connection = match ControlConnection::connect(&control_port).await {
                    Ok(connection) => connection,
                    Err(error) => {
                        observer(TorControlEvent::ControlConnectionFailed(error.to_string()));
                        if cancelled.load(Ordering::SeqCst) {
                            break;
                        }
                        tokio::time::sleep(retry_delay).await;
                        retry_delay = (retry_delay * 2).min(std::time::Duration::from_secs(4));
                        continue;
                    }
                };
                if let Err(error) = connection.subscribe().await {
                    observer(TorControlEvent::ControlConnectionFailed(error.to_string()));
                    if cancelled.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(retry_delay).await;
                    retry_delay = (retry_delay * 2).min(std::time::Duration::from_secs(4));
                    continue;
                }
                retry_delay = std::time::Duration::from_millis(250);
                if let Ok(value) = connection.get_info("network-liveness").await {
                    observer(TorControlEvent::NetworkLiveness(parse_network_liveness(
                        &value,
                    )));
                }
                if let Ok(value) = connection.get_info("status/circuit-established").await {
                    if let Some(established) = parse_circuit_established(&value) {
                        observer(TorControlEvent::CircuitEstablished(established));
                    }
                }

                while !cancelled.load(Ordering::SeqCst) {
                    match connection.next_event().await {
                        Ok(TorControlEvent::CircuitChanged) => {
                            if let Ok(value) =
                                connection.get_info("status/circuit-established").await
                            {
                                if let Some(established) = parse_circuit_established(&value) {
                                    observer(TorControlEvent::CircuitEstablished(established));
                                }
                            }
                        }
                        Ok(event) => observer(event),
                        Err(error) => {
                            observer(TorControlEvent::ControlConnectionFailed(error.to_string()));
                            break;
                        }
                    }
                }
                if !cancelled.load(Ordering::SeqCst) {
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }));
    }

    /// take control conn and drop it.
    /// Closing the owned connection and causes tor daemon to shutdown
    /// Then waits on the Tor daemon thread to exit
    pub fn shutdown(&mut self) -> Result<(), TorErrors> {
        self.shutdown_inner()
    }

    fn shutdown_inner(&mut self) -> Result<(), TorErrors> {
        if let Some(handle) = self.event_handle.take() {
            handle.abort();
        }
        self._ctl.borrow_mut().take();
        if let Some(handle) = self._handle.take() {
            handle.join().map_err(|_| {
                TorErrors::BootStrapError(String::from("Error joining on shutdown"))
            })??;
        }
        Ok(())
    }
}

impl Drop for OwnedTorService {
    fn drop(&mut self) {
        let _ = self.shutdown_inner();
    }
}
/// High level API for Torut used internally by TorService to expose
/// note control functions to FFI and user
impl<F, H> TorControlApi for AuthenticatedConn<TcpStream, H>
where
    H: Fn(AsyncEvent<'static>) -> F,
    F: Future<Output = Result<(), ConnError>>,
{
    fn wait_bootstrap(
        &mut self,
        timeout_ms: Option<u64>,
        observer: Option<TorStatusObserver>,
        cancelled: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = Result<bool, TorErrors>> + '_>> {
        // Wait for boostrap to be done
        let future = async move {
            timeout(
                Duration::from_millis(timeout_ms.unwrap_or(15000)),
                async move {
                    let mut input = String::new();
                    while !input.trim().contains("PROGRESS=100 TAG=done") {
                        if cancelled.load(Ordering::SeqCst) {
                            return Err(TorErrors::StartCancelled);
                        }
                        input = self
                            .get_info("status/bootstrap-phase")
                            .await
                            .map_err(TorErrors::ControlConnectionError)?;
                        if let (Some(observer), Ok(status)) =
                            (observer.as_ref(), parse_bootstrap_status(&input))
                        {
                            observer(TorControlEvent::Bootstrap(status));
                        }
                        if !input.trim().contains("PROGRESS=100 TAG=done") {
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        }
                    }
                    Ok(true)
                },
            )
            .compat()
            .await
            .map_err(|_| TorErrors::BootstrapTimeout)?
        }
        .compat();
        Box::pin(future)
    }
    fn get_status(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<OwnedTorServiceBootstrapPhase, TorErrors>> + '_>> {
        // Wait for boostrap to be done
        Box::pin(
            async move {
                let input = self
                    .get_info("status/bootstrap-phase")
                    .compat()
                    .await
                    .map_err(TorErrors::ControlConnectionError)?;
                if input.trim().contains("TAG=done") {
                    Ok(OwnedTorServiceBootstrapPhase::Done)
                } else {
                    Ok(OwnedTorServiceBootstrapPhase::Other(BootstrapPhase(
                        input.trim().into(),
                    )))
                }
            }
            .compat(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::convert::TryInto;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::thread;

    fn owned_service_without_bootstrap(socks_port: u16, data_dir: &str) -> OwnedTorService {
        let service = TorService::new(TorServiceParam {
            socks_port: Some(socks_port),
            data_dir: data_dir.to_string(),
            bootstrap_timeout_ms: Some(45_000),
        })
        .unwrap();
        let connection = ensure_runtime()
            .block_on(
                async {
                    let mut connection = service
                        .get_control_auth_conn(Some(Box::new(handler) as F))
                        .compat()
                        .await?;
                    connection
                        .take_ownership()
                        .compat()
                        .await
                        .map_err(TorErrors::ControlConnectionError)?;
                    Ok::<_, TorErrors>(connection)
                }
                .compat(),
            )
            .unwrap();

        OwnedTorService {
            socks_port: service.socks_port,
            control_port: service.control_port,
            _handle: service._handle,
            _ctl: RefCell::new(Some(connection)),
            event_handle: None,
        }
    }

    #[test]
    fn normalizes_onion_address_for_control_commands() {
        assert_eq!(normalize_onion_service_id("example.onion:80"), "example");
        assert_eq!(normalize_onion_service_id("example.onion"), "example");
    }

    #[test]
    fn connectivity_query_times_out_when_control_stalls() {
        let cookie_path = std::env::temp_dir().join(format!(
            "react-native-nitro-tor-connectivity-cookie-{}",
            std::process::id()
        ));
        fs::write(&cookie_path, [9_u8; 32]).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let cookie_path_for_server = cookie_path.clone();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut command = String::new();
            reader.read_line(&mut command).unwrap();
            write!(
                stream,
                "250-PROTOCOLINFO 1\r\n250-AUTH METHODS=COOKIE COOKIEFILE=\"{}\"\r\n250-VERSION Tor=\"0.4.9.11\"\r\n250 OK\r\n",
                cookie_path_for_server.display()
            )
            .unwrap();
            stream.flush().unwrap();
            command.clear();
            reader.read_line(&mut command).unwrap();
            stream.write_all(b"250 OK\r\n").unwrap();
            stream.flush().unwrap();
            command.clear();
            reader.read_line(&mut command).unwrap();
            assert_eq!(command, "GETINFO network-liveness\r\n");
            thread::sleep(std::time::Duration::from_millis(150));
        });

        let result = query_connectivity(&address, Duration::from_millis(50));

        assert!(matches!(result, Err(TorErrors::ControlTimeout)));
        server.join().unwrap();
        fs::remove_file(cookie_path).unwrap();
    }

    #[test]
    #[serial(tor)]
    fn dropping_owned_service_waits_for_tor_shutdown() {
        let first =
            owned_service_without_bootstrap(19101, "/tmp/react-native-nitro-tor-drop-first");
        drop(first);

        let mut second =
            owned_service_without_bootstrap(19102, "/tmp/react-native-nitro-tor-drop-second");
        second.shutdown().unwrap();
    }

    #[test]
    #[serial(tor)]
    #[ignore = "requires a live Tor network"]
    fn from_param_and_await_boostrap() {
        ensure_runtime().block_on(
            async {
                let service: TorService = TorServiceParam {
                    socks_port: Some(19051),
                    data_dir: String::from("/tmp/torlib2"),
                    bootstrap_timeout_ms: Some(45000),
                }
                .try_into()
                .unwrap();
                assert_eq!(service.socks_port, 19051);
                assert_eq!(service.control_port.contains("127.0.0.1:"), true);
                assert_eq!(service._handle.is_some(), true);
                let mut control_conn = service
                    .get_control_auth_conn(Some(handler))
                    .compat()
                    .await
                    .unwrap();
                let bootsraped = control_conn
                    .wait_bootstrap(Some(20000), None, Arc::new(AtomicBool::new(false)))
                    .compat()
                    .await
                    .unwrap();
                assert_eq!(bootsraped, true);
                control_conn.take_ownership().await.unwrap();
                drop(control_conn);
                let _ = service._handle.unwrap().join();
            }
            .compat(),
        );
    }

    #[test]
    #[serial(tor)]
    fn bootstrap_timeout() {
        let result = OwnedTorService::new(TorServiceParam {
            socks_port: Some(19051),
            data_dir: String::from("/tmp/torlib-bootstrap-timeout"),
            bootstrap_timeout_ms: Some(500),
        });
        assert!(matches!(result, Err(TorErrors::BootstrapTimeout)));
    }

    #[test]
    #[serial(tor)]
    fn daemon_can_start_again_after_bootstrap_timeout() {
        for (socks_port, data_dir) in [
            (19061, "/tmp/torlib-retry-first"),
            (19062, "/tmp/torlib-retry-second"),
        ] {
            let result = OwnedTorService::new(TorServiceParam {
                socks_port: Some(socks_port),
                data_dir: data_dir.to_string(),
                bootstrap_timeout_ms: Some(100),
            });
            assert!(matches!(result, Err(TorErrors::BootstrapTimeout)));
        }
    }

    #[test]
    #[serial(tor)]
    #[ignore = "requires a live external onion service"]
    fn to_owned() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/torlib2"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let client = utils::get_proxied_client(service.socks_port).unwrap();

        let mut owned_node = service.into_owned_node().unwrap();

        ensure_runtime().block_on(
            async {
                let resp = client
                    .get("http://keybase5wmilwokqirssclfnsqrjdsi7jdir5wy7y7iu3tanwmtp6oid.onion")
                    .send()
                    .await
                    .unwrap();
                assert_eq!(resp.status(), 200);
            }
            .compat(),
        );
        // take ctl and drop it
        owned_node.shutdown().unwrap();
    }

    #[test]
    #[serial(tor)]
    #[ignore = "legacy timing-dependent bootstrap test"]
    fn to_owned_with_timeout() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk/"),
            bootstrap_timeout_ms: Some(30000),
        }
        .try_into()
        .unwrap();
        assert_eq!(service.into_owned_node().is_err(), true);
    }

    #[test]
    #[serial(tor)]
    #[ignore = "requires a live Tor network"]
    fn get_status() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let mut owned_node = service.into_owned_node().unwrap();
        let status = owned_node.get_status().unwrap();
        assert!(matches!(status, OwnedTorServiceBootstrapPhase::Done));
        owned_node.shutdown().unwrap();
    }
    #[test]
    #[serial(tor)]
    #[ignore = "requires a live Tor network"]
    fn create_hidden_service() {
        let service: TorService = TorServiceParam {
            socks_port: Some(19054),
            data_dir: String::from("/tmp/sifir_rs_sdk"),
            bootstrap_timeout_ms: Some(45000),
        }
        .try_into()
        .unwrap();
        let client = utils::get_proxied_client(service.socks_port).unwrap();
        let mut owned_node = service.into_owned_node().unwrap();
        let service_key = owned_node
            .create_hidden_service(TorHiddenServiceParam {
                to_port: 20000,
                hs_port: 20011,
                secret_key: None,
            })
            .unwrap();
        assert!(service_key.onion_url.to_string().contains(".onion"));

        // Spawn a lsner to our request and respond with 200
        let handle = ensure_runtime().spawn(async {
            let listener = TcpListener::bind("127.0.0.1:20000").unwrap();
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let response = "HTTP/1.1 200 OK\r\n\r\n";
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
        });

        let mut onion_url =
            utils::reqwest::Url::parse(&format!("http://{}", service_key.onion_url)).unwrap();
        let _ = onion_url.set_port(Some(20011 as u16));

        ensure_runtime().block_on(
            async {
                let resp = client.get(onion_url).send().await.unwrap();
                assert_eq!(resp.status(), 200);
            }
            .compat(),
        );
        handle.abort();
        owned_node.shutdown().unwrap();
    }
}
