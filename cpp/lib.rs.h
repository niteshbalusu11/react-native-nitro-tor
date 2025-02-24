#pragma once
#include <array>
#include <cstdint>
#include <string>
#include <type_traits>

namespace rust {
  inline namespace cxxbridge1 {
    // #include "rust/cxx.h"

    struct unsafe_bitcopy_t;

    namespace {
      template <typename T> class impl;
    } // namespace

#ifndef CXXBRIDGE1_RUST_STRING
#define CXXBRIDGE1_RUST_STRING
    class String final {
    public:
      String() noexcept;
      String(const String &) noexcept;
      String(String &&) noexcept;
      ~String() noexcept;

      String(const std::string &);
      String(const char *);
      String(const char *, std::size_t);
      String(const char16_t *);
      String(const char16_t *, std::size_t);
#if __cplusplus >= 202002L
      String(const char8_t *s);
      String(const char8_t *s, std::size_t len);
#endif

      static String lossy(const std::string &) noexcept;
      static String lossy(const char *) noexcept;
      static String lossy(const char *, std::size_t) noexcept;
      static String lossy(const char16_t *) noexcept;
      static String lossy(const char16_t *, std::size_t) noexcept;

      String &operator=(const String &) & noexcept;
      String &operator=(String &&) & noexcept;

      explicit operator std::string() const;

      const char *data() const noexcept;
      std::size_t size() const noexcept;
      std::size_t length() const noexcept;
      bool empty() const noexcept;

      const char *c_str() noexcept;

      std::size_t capacity() const noexcept;
      void reserve(size_t new_cap) noexcept;

      using iterator = char *;
      iterator begin() noexcept;
      iterator end() noexcept;

      using const_iterator = const char *;
      const_iterator begin() const noexcept;
      const_iterator end() const noexcept;
      const_iterator cbegin() const noexcept;
      const_iterator cend() const noexcept;

      bool operator==(const String &) const noexcept;
      bool operator!=(const String &) const noexcept;
      bool operator<(const String &) const noexcept;
      bool operator<=(const String &) const noexcept;
      bool operator>(const String &) const noexcept;
      bool operator>=(const String &) const noexcept;

      void swap(String &) noexcept;

      String(unsafe_bitcopy_t, const String &) noexcept;

    private:
      struct lossy_t;
      String(lossy_t, const char *, std::size_t) noexcept;
      String(lossy_t, const char16_t *, std::size_t) noexcept;
      friend void swap(String &lhs, String &rhs) noexcept { lhs.swap(rhs); }

      std::array<std::uintptr_t, 3> repr;
    };
#endif // CXXBRIDGE1_RUST_STRING

#ifndef CXXBRIDGE1_RUST_STR
#define CXXBRIDGE1_RUST_STR
    class Str final {
    public:
      Str() noexcept;
      Str(const String &) noexcept;
      Str(const std::string &);
      Str(const char *);
      Str(const char *, std::size_t);

      Str &operator=(const Str &) & noexcept = default;

      explicit operator std::string() const;

      const char *data() const noexcept;
      std::size_t size() const noexcept;
      std::size_t length() const noexcept;
      bool empty() const noexcept;

      Str(const Str &) noexcept = default;
      ~Str() noexcept = default;

      using iterator = const char *;
      using const_iterator = const char *;
      const_iterator begin() const noexcept;
      const_iterator end() const noexcept;
      const_iterator cbegin() const noexcept;
      const_iterator cend() const noexcept;

      bool operator==(const Str &) const noexcept;
      bool operator!=(const Str &) const noexcept;
      bool operator<(const Str &) const noexcept;
      bool operator<=(const Str &) const noexcept;
      bool operator>(const Str &) const noexcept;
      bool operator>=(const Str &) const noexcept;

      void swap(Str &) noexcept;

    private:
      class uninit;
      Str(uninit) noexcept;
      friend impl<Str>;

      std::array<std::uintptr_t, 2> repr;
    };
#endif // CXXBRIDGE1_RUST_STR
  } // namespace cxxbridge1
} // namespace rust

namespace tor {
  struct HiddenServiceResponse;
  struct StartTorResponse;
} // namespace tor

namespace tor {
#ifndef CXXBRIDGE1_STRUCT_tor$HiddenServiceResponse
#define CXXBRIDGE1_STRUCT_tor$HiddenServiceResponse
  struct HiddenServiceResponse final {
    bool is_success;
    ::rust::String onion_address;
    ::rust::String control;

    using IsRelocatable = ::std::true_type;
  };
#endif // CXXBRIDGE1_STRUCT_tor$HiddenServiceResponse

#ifndef CXXBRIDGE1_STRUCT_tor$StartTorResponse
#define CXXBRIDGE1_STRUCT_tor$StartTorResponse
  struct StartTorResponse final {
    bool is_success;
    ::rust::String onion_address;
    ::rust::String control;
    ::rust::String error_message;

    using IsRelocatable = ::std::true_type;
  };
#endif // CXXBRIDGE1_STRUCT_tor$StartTorResponse

  bool initialize_tor_library() noexcept;

  bool init_tor_service(::std::uint16_t socks_port, ::rust::Str data_dir,
                        ::std::uint64_t timeout_ms) noexcept;

  ::tor::HiddenServiceResponse
  create_hidden_service(::std::uint16_t port, ::std::uint16_t target_port,
                        ::std::array<::std::uint8_t, 64> const &key_data, bool has_key) noexcept;

  ::tor::StartTorResponse start_tor_if_not_running(::rust::Str data_dir,
                                                   ::std::array<::std::uint8_t, 64> const &key_data,
                                                   bool has_key, ::std::uint16_t socks_port,
                                                   ::std::uint16_t target_port,
                                                   ::std::uint64_t timeout_ms) noexcept;

  ::std::int32_t get_service_status() noexcept;

  bool delete_hidden_service(::rust::Str address) noexcept;

  bool shutdown_service() noexcept;
} // namespace tor
