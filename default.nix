{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "bim-env";

  buildInputs = with pkgs; [
        rustc
        cargo
        clang
  ];

  shellHook = ''
    CC=clang
  '';
}
