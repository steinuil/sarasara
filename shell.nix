{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    pkgs.rnix-lsp
    cargo
    rust-analyzer
    pkgconfig
    openssl
    rustc
    rustfmt
  ];
}
