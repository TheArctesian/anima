{ pkgs }:
pkgs.stdenv.mkDerivation {
  pname = "anima-link-preview";
  version = "0.1.0";
  src = ./payload;
  dontBuild = true;
  installPhase = ''
    mkdir -p $out/bin
    install -m755 anima-link $out/bin/anima-link
  '';
}
