{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  name = "abpl-dev";

  packages = with pkgs; [
    # Rust toolchain
    cargo
    rustc
    rustfmt
    
    # Playground
    evcxr
    cargo-expand

    # rust-analyzer
    rust-analyzer

    # uncomment below if we need native dependencies for some reason
    # rustPlatform.rustLibSrc
    # pkg-config
  ];
  # more stuff for rust-analyzer
  RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

  # I don't know why, I don't want to know why, I shouldn't have to wonder why.
  # But tmpdir doesn't exist unless I do this terribleness. All 3 lines.
  shellHook = ''
    bash -c 'mkdir -p $TMPDIR' &
    bash -c 'sleep 1 && mkdir -p $TMPDIR' &
    mkdir -p $TMPDIR || true
  '';
}
