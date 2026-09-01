import { readFileSync, writeFileSync } from 'node:fs';

function update(path, transform) {
  const before = readFileSync(path, 'utf8');
  const after = transform(before).replace(/[\t ]+$/gm, '');
  if (after === before) {
    return;
  }
  writeFileSync(path, after);
}

function replaceRequired(source, search, replacement, path) {
  if (!source.includes(search)) {
    throw new Error(`Craby output changed; unable to patch ${path}`);
  }
  return source.replace(search, replacement);
}

const generatedRust = 'crates/lib/src/generated.rs';
update(generatedRust, (source) => {
  source = source.replaceAll('&mut self', '&self');
  return replaceRequired(
    source,
    `                unsafe {\n                    manager.emit(self.id(), "onStatusChange", signal_ptr);\n                }`,
    `                let delivered = unsafe { manager.emit(self.id(), "onStatusChange", signal_ptr) };\n                if !delivered {\n                    unsafe {\n                        drop(Box::from_raw(signal_ptr));\n                    }\n                }`,
    generatedRust,
  );
});

const ffiRust = 'crates/lib/src/ffi.rs';
update(ffiRust, (source) => {
  source = source.replaceAll('&mut ReactNativeNitroTor', '&ReactNativeNitroTor');
  return replaceRequired(
    source,
    'unsafe fn emit(self: &SignalManager, id: usize, name: &str, signal: *mut ReactNativeNitroTorSignal);',
    'unsafe fn emit(self: &SignalManager, id: usize, name: &str, signal: *mut ReactNativeNitroTorSignal) -> bool;',
    ffiRust,
  );
});

const moduleCpp = 'cpp/CxxReactNativeNitroTorModule.cpp';
update(moduleCpp, (source) => {
  source = replaceRequired(
    source,
    '  threadPool_ = std::make_shared<craby::reactnativenitrotor::utils::ThreadPool>(10);',
    '  threadPool_ = std::make_shared<craby::reactnativenitrotor::utils::ThreadPool>(10);\n  lifecycleThreadPool_ = std::make_shared<craby::reactnativenitrotor::utils::ThreadPool>(1);',
    moduleCpp,
  );
  source = replaceRequired(
    source,
    '  // Shutdown thread pool\n  threadPool_->shutdown();',
    '  // Shutdown thread pool\n  lifecycleThreadPool_->shutdown();\n  threadPool_->shutdown();',
    moduleCpp,
  );
  source = replaceRequired(
    source,
    '  listenersMap_.clear();',
    '  {\n    std::lock_guard<std::mutex> lock(listenersMutex_);\n    listenersMap_.clear();\n  }',
    moduleCpp,
  );
  source = replaceRequired(
    source,
    `    if (thisModule.listenersMap_.find(name) == thisModule.listenersMap_.end()) {\n      thisModule.listenersMap_[name] = std::unordered_map<size_t, std::shared_ptr<facebook::jsi::Function>>();\n    }\n\n    {`,
    '    {',
    moduleCpp,
  );
  const stopStart = source.indexOf('jsi::Value CxxReactNativeNitroTorModule::stop(');
  const stopEnd = source.indexOf('\n}\n\njsi::Value', stopStart);
  if (stopStart < 0 || stopEnd < 0) {
    throw new Error(`Craby output changed; unable to patch ${moduleCpp}`);
  }
  const stopMethod = source.slice(stopStart, stopEnd);
  const patchedStopMethod = replaceRequired(
    stopMethod,
    '    thisModule.threadPool_->enqueue([it_, promise]() mutable {',
    '    thisModule.lifecycleThreadPool_->enqueue([it_, promise]() mutable {',
    moduleCpp,
  );
  return source.slice(0, stopStart) + patchedStopMethod + source.slice(stopEnd);
});

const moduleHpp = 'cpp/CxxReactNativeNitroTorModule.hpp';
update(moduleHpp, (source) => {
  return replaceRequired(
    source,
    '  std::shared_ptr<craby::reactnativenitrotor::utils::ThreadPool> threadPool_;',
    '  std::shared_ptr<craby::reactnativenitrotor::utils::ThreadPool> threadPool_;\n  std::shared_ptr<craby::reactnativenitrotor::utils::ThreadPool> lifecycleThreadPool_;',
    moduleHpp,
  );
});

const androidCmake = 'android/CMakeLists.txt';
update(androidCmake, (source) => {
  return replaceRequired(
    source,
    `target_link_libraries(cxx-react-native-nitro-tor\n  # android\n  ReactAndroid::reactnative`,
    `target_link_libraries(cxx-react-native-nitro-tor\n  # android\n  log\n  ReactAndroid::reactnative`,
    androidCmake,
  );
});

for (const path of [
  'crates/lib/include/CrabySignals.h',
  'ios/include/CrabySignals.h',
  'android/src/main/jni/include/CrabySignals.h',
]) {
  update(path, (source) => {
    if (source.includes('  void emit(')) {
      source = source.replace('  void emit(', '  bool emit(');
      source = replaceRequired(
        source,
        `      it->second(std::string(name), reinterpret_cast<void*>(signal));\n    }\n  }`,
        `      it->second(std::string(name), reinterpret_cast<void*>(signal));\n      return true;\n    }\n    return false;\n  }`,
        path,
      );
    } else if (!source.includes('  bool emit(')) {
      throw new Error(`Craby output changed; unable to patch ${path}`);
    }
    if (path === 'crates/lib/include/CrabySignals.h') {
      source = source.replace(
        '#include "rust/cxx.h"',
        '#if __has_include("rust/cxx.h")\n#include "rust/cxx.h"\n#else\n#include "cxx.h"\n#endif',
      );
    }
    return source;
  });
}
