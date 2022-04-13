let
  pkgs = import <nixos-unstable> {};
in pkgs.mkShell {
  buildInputs = [
    pkgs.cargo 
    pkgs.rustc
    pkgs.rustfmt
    pkgs.clippy
    pkgs.rust-analyzer
    pkgs.cmake
    pkgs.python3
    pkgs.gcc
  ];
}
