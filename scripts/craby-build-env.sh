#!/usr/bin/env sh
set -eu

if command -v xcrun >/dev/null 2>&1; then
  export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-15.1}"

  iphoneos_cc="$(xcrun --sdk iphoneos --find clang)"
  iphoneos_cxx="$(xcrun --sdk iphoneos --find clang++)"
  iphoneos_ar="$(xcrun --sdk iphoneos --find ar)"
  iphoneos_ranlib="$(xcrun --sdk iphoneos --find ranlib)"

  iphonesim_cc="$(xcrun --sdk iphonesimulator --find clang)"
  iphonesim_cxx="$(xcrun --sdk iphonesimulator --find clang++)"
  iphonesim_ar="$(xcrun --sdk iphonesimulator --find ar)"
  iphonesim_ranlib="$(xcrun --sdk iphonesimulator --find ranlib)"

  export CC_aarch64_apple_ios="$iphoneos_cc"
  export CXX_aarch64_apple_ios="$iphoneos_cxx"
  export AR_aarch64_apple_ios="$iphoneos_ar"
  export RANLIB_aarch64_apple_ios="$iphoneos_ranlib"
  export CFLAGS_aarch64_apple_ios="${CFLAGS_aarch64_apple_ios:-} -miphoneos-version-min=$IPHONEOS_DEPLOYMENT_TARGET"
  export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$iphoneos_cc"

  export CC_aarch64_apple_ios_sim="$iphonesim_cc"
  export CXX_aarch64_apple_ios_sim="$iphonesim_cxx"
  export AR_aarch64_apple_ios_sim="$iphonesim_ar"
  export RANLIB_aarch64_apple_ios_sim="$iphonesim_ranlib"
  export CFLAGS_aarch64_apple_ios_sim="${CFLAGS_aarch64_apple_ios_sim:-} -mios-simulator-version-min=$IPHONEOS_DEPLOYMENT_TARGET"
  export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER="$iphonesim_cc"

  export CC_x86_64_apple_ios="$iphonesim_cc"
  export CXX_x86_64_apple_ios="$iphonesim_cxx"
  export AR_x86_64_apple_ios="$iphonesim_ar"
  export RANLIB_x86_64_apple_ios="$iphonesim_ranlib"
  export CFLAGS_x86_64_apple_ios="${CFLAGS_x86_64_apple_ios:-} -mios-simulator-version-min=$IPHONEOS_DEPLOYMENT_TARGET"
  export CARGO_TARGET_X86_64_APPLE_IOS_LINKER="$iphonesim_cc"
fi

if [ "${CARGO_TARGET_DIR+x}" = "x" ] && [ -z "$CARGO_TARGET_DIR" ]; then
  unset CARGO_TARGET_DIR
fi

export ac_cv_func_pipe2=no

exec "$@"
