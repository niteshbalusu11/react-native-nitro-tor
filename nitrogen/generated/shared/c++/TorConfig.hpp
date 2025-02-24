///
/// TorConfig.hpp
/// This file was generated by nitrogen. DO NOT MODIFY THIS FILE.
/// https://github.com/mrousavy/nitro
/// Copyright © 2025 Marc Rousavy @ Margelo
///

#pragma once

#if __has_include(<NitroModules/JSIConverter.hpp>)
#include <NitroModules/JSIConverter.hpp>
#else
#error NitroModules cannot be found! Are you sure you installed NitroModules properly?
#endif
#if __has_include(<NitroModules/NitroDefines.hpp>)
#include <NitroModules/NitroDefines.hpp>
#else
#error NitroModules cannot be found! Are you sure you installed NitroModules properly?
#endif



#include <string>

namespace margelo::nitro::nitrotor {

  /**
   * A struct which can be represented as a JavaScript object (TorConfig).
   */
  struct TorConfig {
  public:
    double socks_port     SWIFT_PRIVATE;
    std::string data_dir     SWIFT_PRIVATE;
    double timeout_ms     SWIFT_PRIVATE;

  public:
    TorConfig() = default;
    explicit TorConfig(double socks_port, std::string data_dir, double timeout_ms): socks_port(socks_port), data_dir(data_dir), timeout_ms(timeout_ms) {}
  };

} // namespace margelo::nitro::nitrotor

namespace margelo::nitro {

  using namespace margelo::nitro::nitrotor;

  // C++ TorConfig <> JS TorConfig (object)
  template <>
  struct JSIConverter<TorConfig> final {
    static inline TorConfig fromJSI(jsi::Runtime& runtime, const jsi::Value& arg) {
      jsi::Object obj = arg.asObject(runtime);
      return TorConfig(
        JSIConverter<double>::fromJSI(runtime, obj.getProperty(runtime, "socks_port")),
        JSIConverter<std::string>::fromJSI(runtime, obj.getProperty(runtime, "data_dir")),
        JSIConverter<double>::fromJSI(runtime, obj.getProperty(runtime, "timeout_ms"))
      );
    }
    static inline jsi::Value toJSI(jsi::Runtime& runtime, const TorConfig& arg) {
      jsi::Object obj(runtime);
      obj.setProperty(runtime, "socks_port", JSIConverter<double>::toJSI(runtime, arg.socks_port));
      obj.setProperty(runtime, "data_dir", JSIConverter<std::string>::toJSI(runtime, arg.data_dir));
      obj.setProperty(runtime, "timeout_ms", JSIConverter<double>::toJSI(runtime, arg.timeout_ms));
      return obj;
    }
    static inline bool canConvert(jsi::Runtime& runtime, const jsi::Value& value) {
      if (!value.isObject()) {
        return false;
      }
      jsi::Object obj = value.getObject(runtime);
      if (!JSIConverter<double>::canConvert(runtime, obj.getProperty(runtime, "socks_port"))) return false;
      if (!JSIConverter<std::string>::canConvert(runtime, obj.getProperty(runtime, "data_dir"))) return false;
      if (!JSIConverter<double>::canConvert(runtime, obj.getProperty(runtime, "timeout_ms"))) return false;
      return true;
    }
  };

} // namespace margelo::nitro
