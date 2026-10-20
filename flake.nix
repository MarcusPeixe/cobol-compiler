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
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            rust-analyzer
            clippy
            rustfmt
            typst
          ];
        };
      });
}
