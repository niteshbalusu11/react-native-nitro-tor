use craby::{prelude::*, throw};

use crate::ffi::bridging::*;
use crate::generated::*;
use crate::tor;

pub struct ReactNativeNitroTor {
    ctx: Context,
}

#[craby_module]
impl ReactNativeNitroTorSpec for ReactNativeNitroTor {
    fn create_hidden_service(
        &mut self,
        params: HiddenServiceParams,
    ) -> Promise<HiddenServiceResponse> {
        Ok(tor::create_hidden_service(params.port, params.target_port))
    }

    fn delete_hidden_service(&mut self, onion_address: &str) -> Promise<Boolean> {
        let address = onion_address.to_string();
        Ok(tor::delete_hidden_service(address))
    }

    fn get_service_status(&mut self) -> Promise<Number> {
        Ok(tor::get_service_status())
    }

    fn http_delete(&mut self, params: HttpDeleteParams) -> Promise<HttpResponse> {
        Ok(tor::http_delete(
            params.url,
            params.headers,
            params.timeout_ms,
        ))
    }

    fn http_get(&mut self, params: HttpGetParams) -> Promise<HttpResponse> {
        Ok(tor::http_get(params.url, params.headers, params.timeout_ms))
    }

    fn http_post(&mut self, params: HttpPostParams) -> Promise<HttpResponse> {
        Ok(tor::http_post(
            params.url,
            params.body,
            params.headers,
            params.timeout_ms,
        ))
    }

    fn http_put(&mut self, params: HttpPutParams) -> Promise<HttpResponse> {
        Ok(tor::http_put(
            params.url,
            params.body,
            params.headers,
            params.timeout_ms,
        ))
    }

    fn init_tor_service(&mut self, config: TorConfig) -> Promise<Boolean> {
        Ok(tor::init_tor_service(
            config.socks_port,
            config.data_dir,
            config.timeout_ms,
        ))
    }

    fn shutdown_service(&mut self) -> Promise<Boolean> {
        Ok(tor::shutdown_service())
    }

    fn start_tor_if_not_running(&mut self, params: StartTorParams) -> Promise<StartTorResponse> {
        Ok(tor::start_tor_if_not_running(
            params.data_dir,
            params.socks_port,
            params.target_port,
            params.timeout_ms,
        ))
    }
}
