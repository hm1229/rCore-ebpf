{
  inputs = {
    nixpkgs.url = "github:NickCao/nixpkgs/nixos-unstable-small";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlay ];
          };
        in
        {
          devShell = pkgs.mkShell {
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            buildInputs = with pkgs; [
              wget
              qemu
              ((rust-bin.fromRustupToolchainFile ./rust-toolchain).override {
                extensions = [ "rust-src" "llvm-tools-preview" "rust-analyzer-preview" ];
              })
              linuxHeaders
              rust-bindgen
            ];
          };
        }
      );
}
