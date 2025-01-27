{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  inputsFrom = [ (pkgs.callPackage ./default.nix { }) ];
  buildInputs = with pkgs; [
    rust-analyzer # LSP Server
    rustfmt       # Formatter
    clippy        # Linter
  ];
}
