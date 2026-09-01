#pragma once

#if __has_include("cxx.h")
#include "cxx.h"
#else
#include "cxx.h"
#endif
#include <functional>
#include <memory>
#include <mutex>
#include <unordered_map>

namespace craby {
namespace reactnativenitrotor {
namespace bridging {
  struct ReactNativeNitroTorSignal;
}
namespace modules {
  class CxxReactNativeNitroTor;
}
}
}

namespace craby {
namespace reactnativenitrotor {
namespace signals {

using Delegate = std::function<void(const std::string& signalName, void* signal)>;

class SignalManager {
public:
  static SignalManager& getInstance() {
    static SignalManager instance;
    return instance;
  }

  bool emit(uintptr_t id, rust::Str name, craby::reactnativenitrotor::bridging::ReactNativeNitroTorSignal* signal) const {
    std::lock_guard<std::mutex> lock(mutex_);
    auto it = delegates_.find(id);
    if (it != delegates_.end()) {
      it->second(std::string(name), reinterpret_cast<void*>(signal));
      return true;
    }
    return false;
  }

  void registerDelegate(uintptr_t id, Delegate delegate) const {
    std::lock_guard<std::mutex> lock(mutex_);
    delegates_.insert_or_assign(id, delegate);
  }

  void unregisterDelegate(uintptr_t id) const {
    std::lock_guard<std::mutex> lock(mutex_);
    delegates_.erase(id);
  }

private:
  SignalManager() = default;
  mutable std::unordered_map<uintptr_t, Delegate> delegates_;
  mutable std::mutex mutex_;
};

inline const SignalManager& getSignalManager() {
  return SignalManager::getInstance();
}

} // namespace signals
} // namespace reactnativenitrotor
} // namespace craby
