{
  inputs = {
    nixpkgs.url = "github:NickCao/nixpkgs/nixos-unstable-small";
    nixpkgs-qemu.url = "github:NixOS/nixpkgs/9e403b19a1444b373874b6e9efdd728613c6badc";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };
  outputs = { self, nixpkgs, nixpkgs-qemu, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlay ];
          };
          pkgsQemu = import nixpkgs-qemu { inherit system; };
        in
        {
          devShell = pkgs.mkShell {
            LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
            buildInputs = with pkgs; [
              wget
              pkgsQemu.qemu
              ((rust-bin.fromRustupToolchainFile ./rust-toolchain).override {
                extensions = [ "rust-src" "llvm-tools-preview" "rust-analyzer-preview" ];
              })
              cmake
              llvmPackages_13.bintools
              pkgsCross.riscv64.pkgsMusl.buildPackages.bintools
              pkgsCross.riscv64.pkgsMusl.buildPackages.gcc
            ];
          };
        }
      );
}
