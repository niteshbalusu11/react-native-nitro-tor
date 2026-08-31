#![allow(non_camel_case_types)]

//! Builds Tor and its minimal dependencies into a library and exposes Tor's C API.

use std::os::raw::{c_char, c_int, c_void};

type tor_main_configuration_t = c_void;

extern "C" {
    pub fn tor_main_configuration_new() -> *mut tor_main_configuration_t;
    pub fn tor_main_configuration_set_command_line(
        config: *mut tor_main_configuration_t,
        argc: c_int,
        argv: *const *const c_char,
    ) -> c_int;
    pub fn tor_main_configuration_free(config: *mut tor_main_configuration_t);
    pub fn tor_run_main(configuration: *const tor_main_configuration_t) -> c_int;
}
