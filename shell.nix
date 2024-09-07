{ pkgs ? import <nixpkgs> { overlays = [ (import <rust-overlay>) ]; }, ... }:
with pkgs;
mkShell {
  nativeBuildInputs = [ pkg-config rust-bin.stable.latest.default ];
  buildInputs = [ dbus udev openssl ];
}
