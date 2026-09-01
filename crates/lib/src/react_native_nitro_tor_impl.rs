use std::sync::Arc;

use craby::prelude::*;

use crate::ffi::bridging::*;
use crate::generated::*;
use crate::tor;

pub struct ReactNativeNitroTor {
    ctx: Context,
}

impl ReactNativeNitroTor {
    fn register_status_emitter(&self) {
        let id = self.id();
        tor::register_status_emitter(Arc::new(move |payload| {
            let signal = Box::new(ReactNativeNitroTorSignal::OnStatusChange(payload));
            let signal_ptr = Box::into_raw(signal);
            let manager = get_signal_manager();
            let delivered = unsafe { manager.emit(id, "onStatusChange", signal_ptr) };
            if !delivered {
                unsafe {
                    drop(Box::from_raw(signal_ptr));
                }
            }
        }));
    }
}

#[craby_module]
impl ReactNativeNitroTorSpec for ReactNativeNitroTor {
    fn start(&self, config: NativeTorConfig) -> Promise<String> {
        self.register_status_emitter();
        let config = tor::validate_config(
            config.data_directory,
            config.socks_port,
            config.bootstrap_timeout_ms,
        )?;
        tor::start(config)
    }

    fn stop(&self) -> Promise<Void> {
        self.register_status_emitter();
        tor::stop()
    }

    fn get_status(&self) -> Promise<String> {
        self.register_status_emitter();
        tor::get_status()
    }

    fn request_new_identity(&self) -> Promise<Void> {
        self.register_status_emitter();
        tor::request_new_identity()
    }

    fn http_request(&self, request: NativeHttpRequest) -> Promise<String> {
        self.register_status_emitter();
        let timeout_ms = tor::validate_timeout(request.timeout_ms, "timeoutMs", "INVALID_REQUEST")?;
        tor::http_request(tor::HttpRequest {
            url: request.url,
            method: request.method,
            headers_json: request.headers_json,
            body: if request.body.is_empty() {
                None
            } else {
                Some(request.body)
            },
            timeout_ms,
            allow_invalid_certificates: request.allow_invalid_certificates,
        })
    }

    fn create_hidden_service(
        &self,
        options: NativeHiddenServiceOptions,
    ) -> Promise<NativeHiddenService> {
        self.register_status_emitter();
        let virtual_port = tor::validate_port(
            options.virtual_port,
            "virtualPort",
            "INVALID_HIDDEN_SERVICE",
        )?;
        let target_port =
            tor::validate_port(options.target_port, "targetPort", "INVALID_HIDDEN_SERVICE")?;
        let result = tor::create_hidden_service(tor::HiddenServiceOptions {
            virtual_port,
            target_port,
            private_key: options.private_key,
        })?;
        Ok(NativeHiddenService {
            onion_address: result.onion_address,
            private_key: result.private_key,
        })
    }

    fn remove_hidden_service(&self, onion_address: &str) -> Promise<Void> {
        self.register_status_emitter();
        tor::remove_hidden_service(onion_address.to_string())
    }
}
