{ pkgs }:
pkgs.stdenv.mkDerivation {
  pname = "anima-daemon-preview";
  version = "0.1.0";
  src = ./payload;
  nativeBuildInputs = [ pkgs.autoPatchelfHook ];
  buildInputs = [ pkgs.stdenv.cc.cc.lib ];
  dontBuild = true;
  installPhase = ''
    mkdir -p $out/bin $out/share/anima-build
    install -m755 anima-daemon $out/bin/anima-daemon
    cp SOURCE-REVISION CARGO-LOCK-SHA256 BINARY-SHA256 $out/share/anima-build/
  '';
}
