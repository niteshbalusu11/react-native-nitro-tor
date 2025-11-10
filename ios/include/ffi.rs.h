#pragma once
#include <array>
#include <cstddef>
#include <cstdint>
#include <new>
#include <string>
#include <type_traits>
#include <utility>
#if __cplusplus >= 201703L
#include <string_view>
#endif

#ifdef __clang__
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdollar-in-identifier-extension"
#endif // __clang__

namespace rust {
inline namespace cxxbridge1 {
// #include "rust/cxx.h"

struct unsafe_bitcopy_t;

namespace {
template <typename T>
class impl;
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
#ifdef __cpp_char8_t
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
#if __cplusplus >= 201703L
  explicit operator std::string_view() const;
#endif

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

#ifndef CXXBRIDGE1_RUST_BOX
#define CXXBRIDGE1_RUST_BOX
template <typename T>
class Box final {
public:
  using element_type = T;
  using const_pointer =
      typename std::add_pointer<typename std::add_const<T>::type>::type;
  using pointer = typename std::add_pointer<T>::type;

  Box() = delete;
  Box(Box &&) noexcept;
  ~Box() noexcept;

  explicit Box(const T &);
  explicit Box(T &&);

  Box &operator=(Box &&) & noexcept;

  const T *operator->() const noexcept;
  const T &operator*() const noexcept;
  T *operator->() noexcept;
  T &operator*() noexcept;

  template <typename... Fields>
  static Box in_place(Fields &&...);

  void swap(Box &) noexcept;

  static Box from_raw(T *) noexcept;

  T *into_raw() noexcept;

  /* Deprecated */ using value_type = element_type;

private:
  class uninit;
  class allocation;
  Box(uninit) noexcept;
  void drop() noexcept;

  friend void swap(Box &lhs, Box &rhs) noexcept { lhs.swap(rhs); }

  T *ptr;
};

template <typename T>
class Box<T>::uninit {};

template <typename T>
class Box<T>::allocation {
  static T *alloc() noexcept;
  static void dealloc(T *) noexcept;

public:
  allocation() noexcept : ptr(alloc()) {}
  ~allocation() noexcept {
    if (this->ptr) {
      dealloc(this->ptr);
    }
  }
  T *ptr;
};

template <typename T>
Box<T>::Box(Box &&other) noexcept : ptr(other.ptr) {
  other.ptr = nullptr;
}

template <typename T>
Box<T>::Box(const T &val) {
  allocation alloc;
  ::new (alloc.ptr) T(val);
  this->ptr = alloc.ptr;
  alloc.ptr = nullptr;
}

template <typename T>
Box<T>::Box(T &&val) {
  allocation alloc;
  ::new (alloc.ptr) T(std::move(val));
  this->ptr = alloc.ptr;
  alloc.ptr = nullptr;
}

template <typename T>
Box<T>::~Box() noexcept {
  if (this->ptr) {
    this->drop();
  }
}

template <typename T>
Box<T> &Box<T>::operator=(Box &&other) & noexcept {
  if (this->ptr) {
    this->drop();
  }
  this->ptr = other.ptr;
  other.ptr = nullptr;
  return *this;
}

template <typename T>
const T *Box<T>::operator->() const noexcept {
  return this->ptr;
}

template <typename T>
const T &Box<T>::operator*() const noexcept {
  return *this->ptr;
}

template <typename T>
T *Box<T>::operator->() noexcept {
  return this->ptr;
}

template <typename T>
T &Box<T>::operator*() noexcept {
  return *this->ptr;
}

template <typename T>
template <typename... Fields>
Box<T> Box<T>::in_place(Fields &&...fields) {
  allocation alloc;
  auto ptr = alloc.ptr;
  ::new (ptr) T{std::forward<Fields>(fields)...};
  alloc.ptr = nullptr;
  return from_raw(ptr);
}

template <typename T>
void Box<T>::swap(Box &rhs) noexcept {
  using std::swap;
  swap(this->ptr, rhs.ptr);
}

template <typename T>
Box<T> Box<T>::from_raw(T *raw) noexcept {
  Box box = uninit{};
  box.ptr = raw;
  return box;
}

template <typename T>
T *Box<T>::into_raw() noexcept {
  T *raw = this->ptr;
  this->ptr = nullptr;
  return raw;
}

template <typename T>
Box<T>::Box(uninit) noexcept {}
#endif // CXXBRIDGE1_RUST_BOX

#ifndef CXXBRIDGE1_RUST_OPAQUE
#define CXXBRIDGE1_RUST_OPAQUE
class Opaque {
public:
  Opaque() = delete;
  Opaque(const Opaque &) = delete;
  ~Opaque() = delete;
};
#endif // CXXBRIDGE1_RUST_OPAQUE

#ifndef CXXBRIDGE1_IS_COMPLETE
#define CXXBRIDGE1_IS_COMPLETE
namespace detail {
namespace {
template <typename T, typename = std::size_t>
struct is_complete : std::false_type {};
template <typename T>
struct is_complete<T, decltype(sizeof(T))> : std::true_type {};
} // namespace
} // namespace detail
#endif // CXXBRIDGE1_IS_COMPLETE

#ifndef CXXBRIDGE1_LAYOUT
#define CXXBRIDGE1_LAYOUT
class layout {
  template <typename T>
  friend std::size_t size_of();
  template <typename T>
  friend std::size_t align_of();
  template <typename T>
  static typename std::enable_if<std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_size_of() {
    return T::layout::size();
  }
  template <typename T>
  static typename std::enable_if<!std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_size_of() {
    return sizeof(T);
  }
  template <typename T>
  static
      typename std::enable_if<detail::is_complete<T>::value, std::size_t>::type
      size_of() {
    return do_size_of<T>();
  }
  template <typename T>
  static typename std::enable_if<std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_align_of() {
    return T::layout::align();
  }
  template <typename T>
  static typename std::enable_if<!std::is_base_of<Opaque, T>::value,
                                 std::size_t>::type
  do_align_of() {
    return alignof(T);
  }
  template <typename T>
  static
      typename std::enable_if<detail::is_complete<T>::value, std::size_t>::type
      align_of() {
    return do_align_of<T>();
  }
};

template <typename T>
std::size_t size_of() {
  return layout::size_of<T>();
}

template <typename T>
std::size_t align_of() {
  return layout::align_of<T>();
}
#endif // CXXBRIDGE1_LAYOUT
} // namespace cxxbridge1
} // namespace rust

#if __cplusplus >= 201402L
#define CXX_DEFAULT_VALUE(value) = value
#else
#define CXX_DEFAULT_VALUE(value)
#endif

namespace craby {
  namespace reactnativenitrotor {
    namespace bridging {
      struct StartTorResponse;
      struct StartTorParams;
      struct HttpPostParams;
      struct HttpGetParams;
      struct HttpDeleteParams;
      struct HiddenServiceParams;
      struct TorConfig;
      struct HttpResponse;
      struct HttpPutParams;
      struct HiddenServiceResponse;
      struct ReactNativeNitroTor;
    }
  }
}

namespace craby {
namespace reactnativenitrotor {
namespace bridging {
#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$StartTorResponse
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$StartTorResponse
struct StartTorResponse final {
  bool is_success CXX_DEFAULT_VALUE(false);
  ::rust::String onion_address;
  ::rust::String control;
  ::rust::String error_message;

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$StartTorResponse

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$StartTorParams
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$StartTorParams
struct StartTorParams final {
  ::rust::String data_dir;
  double socks_port CXX_DEFAULT_VALUE(0);
  double target_port CXX_DEFAULT_VALUE(0);
  double timeout_ms CXX_DEFAULT_VALUE(0);

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$StartTorParams

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpPostParams
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpPostParams
struct HttpPostParams final {
  ::rust::String url;
  ::rust::String body;
  ::rust::String headers;
  double timeout_ms CXX_DEFAULT_VALUE(0);

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpPostParams

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpGetParams
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpGetParams
struct HttpGetParams final {
  ::rust::String url;
  ::rust::String headers;
  double timeout_ms CXX_DEFAULT_VALUE(0);

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpGetParams

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpDeleteParams
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpDeleteParams
struct HttpDeleteParams final {
  ::rust::String url;
  ::rust::String headers;
  double timeout_ms CXX_DEFAULT_VALUE(0);

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpDeleteParams

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HiddenServiceParams
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HiddenServiceParams
struct HiddenServiceParams final {
  double port CXX_DEFAULT_VALUE(0);
  double target_port CXX_DEFAULT_VALUE(0);

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HiddenServiceParams

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$TorConfig
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$TorConfig
struct TorConfig final {
  double socks_port CXX_DEFAULT_VALUE(0);
  ::rust::String data_dir;
  double timeout_ms CXX_DEFAULT_VALUE(0);

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$TorConfig

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpResponse
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpResponse
struct HttpResponse final {
  double status_code CXX_DEFAULT_VALUE(0);
  ::rust::String body;
  ::rust::String error;

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpResponse

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpPutParams
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpPutParams
struct HttpPutParams final {
  ::rust::String url;
  ::rust::String body;
  ::rust::String headers;
  double timeout_ms CXX_DEFAULT_VALUE(0);

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HttpPutParams

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HiddenServiceResponse
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HiddenServiceResponse
struct HiddenServiceResponse final {
  bool is_success CXX_DEFAULT_VALUE(false);
  ::rust::String onion_address;
  ::rust::String control;

  using IsRelocatable = ::std::true_type;
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$HiddenServiceResponse

#ifndef CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$ReactNativeNitroTor
#define CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$ReactNativeNitroTor
struct ReactNativeNitroTor final : public ::rust::Opaque {
  ~ReactNativeNitroTor() = delete;

private:
  friend ::rust::layout;
  struct layout {
    static ::std::size_t size() noexcept;
    static ::std::size_t align() noexcept;
  };
};
#endif // CXXBRIDGE1_STRUCT_craby$reactnativenitrotor$bridging$ReactNativeNitroTor

::rust::Box<::craby::reactnativenitrotor::bridging::ReactNativeNitroTor> createReactNativeNitroTor(::std::size_t id, ::rust::Str data_path) noexcept;

::craby::reactnativenitrotor::bridging::HiddenServiceResponse createHiddenService(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_, ::craby::reactnativenitrotor::bridging::HiddenServiceParams params);

bool deleteHiddenService(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_, ::rust::Str onion_address);

double getServiceStatus(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_);

::craby::reactnativenitrotor::bridging::HttpResponse httpDelete(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_, ::craby::reactnativenitrotor::bridging::HttpDeleteParams params);

::craby::reactnativenitrotor::bridging::HttpResponse httpGet(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_, ::craby::reactnativenitrotor::bridging::HttpGetParams params);

::craby::reactnativenitrotor::bridging::HttpResponse httpPost(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_, ::craby::reactnativenitrotor::bridging::HttpPostParams params);

::craby::reactnativenitrotor::bridging::HttpResponse httpPut(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_, ::craby::reactnativenitrotor::bridging::HttpPutParams params);

bool initTorService(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_, ::craby::reactnativenitrotor::bridging::TorConfig config);

bool shutdownService(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_);

::craby::reactnativenitrotor::bridging::StartTorResponse startTorIfNotRunning(::craby::reactnativenitrotor::bridging::ReactNativeNitroTor &it_, ::craby::reactnativenitrotor::bridging::StartTorParams params);
} // namespace bridging
} // namespace reactnativenitrotor
} // namespace craby

#ifdef __clang__
#pragma clang diagnostic pop
#endif // __clang__
