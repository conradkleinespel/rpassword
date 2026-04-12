{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustup
  ];
  shellHook = ''
    rustup default stable
    rustup component add rust-src
  '';
}
