{pkgs ? import <nixpkgs> {}}:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustup
    #pkgsCross.mingwW64.stdenv.cc
    wineWowPackages.stable
    emscripten
  ];
  buildInputs = with pkgs; [
    #pkgsCross.mingwW64.windows.pthreads
    nodejs
    podman
  ];
  shellHook = ''
    git config set core.hooksPath githooks

    rustup install 1.85
    rustup default 1.85
    rustup component add rust-src
    rustup target add x86_64-unknown-linux-gnu
    rustup target add x86_64-pc-windows-gnu
    rustup target add wasm32-unknown-emscripten
  '';
}
