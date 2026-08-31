{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    android-nixpkgs = {
      url = "github:tadfisher/android-nixpkgs";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      android-nixpkgs,
    }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;

      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
            android_sdk.accept_license = true;
          };
        };

      androidSdkFor =
        system:
        android-nixpkgs.sdk.${system} (
          sdkPkgs: with sdkPkgs; [
            cmdline-tools-22-0
            build-tools-35-0-0
            build-tools-36-0-0
            build-tools-37-0-0
            platform-tools
            platforms-android-36
            platforms-android-37-0
            ndk-28-2-13676358
            cmake-3-22-1
          ]
        );

      # macOS-specific derivations
      darwinDerivations = {
        xcode-wrapper =
          pkgs:
          pkgs.stdenv.mkDerivation {
            name = "xcode-wrapper-16.4.0";
            buildInputs = [ pkgs.darwin.cctools ];
            buildCommand = ''
              mkdir -p $out/bin

              # Create wrapper scripts instead of symlinks
              cat > $out/bin/xcodebuild << EOF
              #!/bin/sh
              exec /usr/bin/xcodebuild "\$@"
              EOF

              cat > $out/bin/xcrun << EOF
              #!/bin/sh
              exec /usr/bin/xcrun "\$@"
              EOF

              cat > $out/bin/xcode-select << EOF
              #!/bin/sh
              exec /usr/bin/xcode-select "\$@"
              EOF

              cat > $out/bin/codesign << EOF
              #!/bin/sh
              exec /usr/bin/codesign "\$@"
              EOF

              cat > $out/bin/ld << EOF
              #!/bin/sh
              exec /usr/bin/ld "\$@"
              EOF

              cat > $out/bin/clang << EOF
              #!/bin/sh
              exec /usr/bin/clang "\$@"
              EOF

              chmod +x $out/bin/*

              if [ -d "/Applications/Xcode.app" ]; then
                DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
              elif [ -d "/Applications/Xcode-16.4.0.app" ]; then
                DEVELOPER_DIR="/Applications/Xcode-16.4.0.app/Contents/Developer"
              else
                echo "Error: Xcode not found"
                exit 1
              fi

              echo "export DEVELOPER_DIR=\"$DEVELOPER_DIR\"" > $out/bin/env.sh
            '';
          };

        scripts = pkgs: {
          build-ios = pkgs.writeScriptBin "build-ios" ''
            #!${pkgs.stdenv.shell}
            echo "Building for iOS..."
            chmod +x ./build-ios.sh
            ./build-ios.sh
          '';

          build-android = pkgs.writeScriptBin "build-android" ''
            #!${pkgs.stdenv.shell}
            echo "Building for Android..."
            chmod +x ./build-android.sh
            ./build-android.sh
          '';
        };
      };

      # System-specific shell configuration
      mkShellFor =
        system:
        let
          pkgs = pkgsFor system;
          androidSdk = androidSdkFor system;
          scripts = darwinDerivations.scripts pkgs;

          basePackages = with pkgs; [
            yarn-berry_4
            androidSdk
            autoconf
            automake
            libtool
            openssl
            rustup
            protobuf
            nodejs_22
            iconv
            pkg-config
            jdk17
          ];

          darwinPackages = with pkgs; [
            bundler
            cocoapods
            (darwinDerivations.xcode-wrapper pkgs)
            scripts.build-ios
            scripts.build-android
          ];

          darwinHook = ''
            export LC_ALL=en_US.UTF-8
            export LANG=en_US.UTF-8
            export JAVA_HOME="${pkgs.jdk17.home}"
            export ANDROID_HOME="${androidSdk}/share/android-sdk"
            export ANDROID_NDK_HOME="${androidSdk}/share/android-sdk/ndk/28.2.13676358"

            export PATH="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin:$PATH"

            export AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"
            export AS="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-as"
            export NM="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-nm"
            export STRIP="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-strip"

            export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android30-clang"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_I686_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/i686-linux-android30-clang"
            export CARGO_TARGET_I686_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_X86_64_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/x86_64-linux-android30-clang"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/armv7a-linux-androideabi30-clang"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"

            unset SDKROOT
            if [ "''${CARGO_TARGET_DIR+x}" = "x" ] && [ -z "$CARGO_TARGET_DIR" ]; then
              unset CARGO_TARGET_DIR
            fi
            # Tor's Autoconf check finds pipe2 in libc, but iOS does not expose it.
            export ac_cv_func_pipe2=no

            rustup target add aarch64-linux-android x86_64-linux-android i686-linux-android armv7-linux-androideabi
            rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin x86_64-apple-darwin

            if [ -f "${darwinDerivations.xcode-wrapper pkgs}/bin/env.sh" ]; then
              source "${darwinDerivations.xcode-wrapper pkgs}/bin/env.sh"
            fi

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
            export CARGO_TARGET_AARCH64_APPLE_IOS_LINKER="$iphoneos_cc"

            export CC_aarch64_apple_ios_sim="$iphonesim_cc"
            export CXX_aarch64_apple_ios_sim="$iphonesim_cxx"
            export AR_aarch64_apple_ios_sim="$iphonesim_ar"
            export RANLIB_aarch64_apple_ios_sim="$iphonesim_ranlib"
            export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_LINKER="$iphonesim_cc"

            export CC_x86_64_apple_ios="$iphonesim_cc"
            export CXX_x86_64_apple_ios="$iphonesim_cxx"
            export AR_x86_64_apple_ios="$iphonesim_ar"
            export RANLIB_x86_64_apple_ios="$iphonesim_ranlib"
            export CARGO_TARGET_X86_64_APPLE_IOS_LINKER="$iphonesim_cc"

            echo "iOS development environment:"
            echo "DEVELOPER_DIR: $DEVELOPER_DIR"
            xcodebuild -version
          '';

          linuxHook = ''
            export LC_ALL=en_US.UTF-8
            export LANG=en_US.UTF-8
            export JAVA_HOME="${pkgs.jdk17.home}"
            export ANDROID_HOME="${androidSdk}/share/android-sdk"
            export ANDROID_NDK_HOME="${androidSdk}/share/android-sdk/ndk/28.2.13676358"

            export PATH="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"

            export AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"
            export AS="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-as"
            export NM="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm"
            export STRIP="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"

            export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_I686_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/i686-linux-android30-clang"
            export CARGO_TARGET_I686_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_X86_64_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android30-clang"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_AR="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi30-clang"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RANLIB="${androidSdk}/share/android-sdk/ndk/28.2.13676358/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"

            rustup target add aarch64-linux-android x86_64-linux-android i686-linux-android armv7-linux-androideabi
          '';

        in
        pkgs.mkShellNoCC {
          buildInputs = if system == "aarch64-darwin" then basePackages ++ darwinPackages else basePackages;

          shellHook = if system == "aarch64-darwin" then darwinHook else linuxHook;
        };
    in
    {
      devShells = forAllSystems (system: {
        default = mkShellFor system;
      });
    };
}
