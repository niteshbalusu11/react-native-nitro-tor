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
            cmdline-tools-latest
            build-tools-35-0-0
            build-tools-36-0-0
            platform-tools
            platforms-android-35
            platforms-android-36
            ndk-27-1-12297006
            ndk-27-0-12077973
            ndk-26-1-10909125
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

              if [ -d "/Applications/Xcode-beta.app" ]; then
                DEVELOPER_DIR="/Applications/Xcode-beta.app/Contents/Developer"
              elif [ -d "/Applications/Xcode.app" ]; then
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
            export ANDROID_NDK_HOME="${androidSdk}/share/android-sdk/ndk/26.1.10909125"

            export PATH="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin:$PATH"

            export AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"
            export AS="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-as"
            export NM="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-nm"
            export STRIP="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-strip"

            export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android30-clang"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_I686_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/i686-linux-android30-clang"
            export CARGO_TARGET_I686_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_X86_64_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/x86_64-linux-android30-clang"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/armv7a-linux-androideabi30-clang"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ranlib"

            unset SDKROOT

            rustup target add aarch64-linux-android x86_64-linux-android i686-linux-android armv7-linux-androideabi
            rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim aarch64-apple-darwin x86_64-apple-darwin

            if [ -f "${darwinDerivations.xcode-wrapper pkgs}/bin/env.sh" ]; then
              source "${darwinDerivations.xcode-wrapper pkgs}/bin/env.sh"
            fi

            export LD=/usr/bin/clang
            export LD_FOR_TARGET=/usr/bin/clang

            echo "iOS development environment:"
            echo "DEVELOPER_DIR: $DEVELOPER_DIR"
            xcodebuild -version
          '';

          linuxHook = ''
            export LC_ALL=en_US.UTF-8
            export LANG=en_US.UTF-8
            export JAVA_HOME="${pkgs.jdk17.home}"
            export ANDROID_HOME="${androidSdk}/share/android-sdk"
            export ANDROID_NDK_HOME="${androidSdk}/share/android-sdk/ndk/26.1.10909125"

            export PATH="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH"

            export AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"
            export AS="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-as"
            export NM="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm"
            export STRIP="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip"

            export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang"
            export CARGO_TARGET_AARCH64_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_I686_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/i686-linux-android30-clang"
            export CARGO_TARGET_I686_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_X86_64_LINUX_ANDROID_AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android30-clang"
            export CARGO_TARGET_X86_64_LINUX_ANDROID_RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"

            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_AR="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi30-clang"
            export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_RANLIB="${androidSdk}/share/android-sdk/ndk/26.1.10909125/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ranlib"

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
