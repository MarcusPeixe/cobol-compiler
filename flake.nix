{
  description = "COBOL Compiler";

  inputs = {
    # nixpkgs for darwin
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # flake-utils to simplify multiple systems
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # import nixpkgs for this system
        pkgs = import nixpkgs {
          inherit system;
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rust-analyzer
            clippy
            rustfmt
          ];
        };
      });
}
