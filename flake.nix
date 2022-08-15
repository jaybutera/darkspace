{
  description = "Darkspace Flake";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-22.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.mozilla = { url = "github:mozilla/nixpkgs-mozilla"; flake = false; };

  outputs =
    { self
    , nixpkgs
    , mozilla
    , flake-utils
    , ...
    } @inputs:
    let rustOverlay = final: prev:
          let rustChannel = prev.rustChannelOf {
            channel = "1.63.0";
            sha256 = "sha256-KXx+ID0y4mg2B3LHp7IyaiMrdexF6octADnAtFIOjrY=";
          };
          in
          { inherit rustChannel;
            rustc = rustChannel.rust;
            cargo = rustChannel.rust;
          };
    in flake-utils.lib.eachDefaultSystem
      (system:
        let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import "${mozilla}/rust-overlay.nix")
            rustOverlay
          ];
        };
        in {
          devShell = pkgs.mkShell {
            #RUST_BACKTRACE=1;
            #RUST_LOG="trace";
            buildInputs = with pkgs; [
              openssl
              gnumake
              openssl
              clang
              libclang
              pkg-config
              cmake
              llvm
              freetype
              fontconfig
              sfml
              (rustChannel.rust.override { extensions = [ "rust-src" ]; })
            ];
            LIBCLANG_PATH="${pkgs.libclang.lib}/lib";
          };
        });
}
