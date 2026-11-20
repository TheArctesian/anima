{ pkgs, lib, ... }:
let
  display = pkgs.writeShellApplication {
    name = "anima-display";
    runtimeInputs = with pkgs; [ python3 xrandr xinput systemd zenity ];
    text = ''exec python3 ${./display/orientation.py} "$@"'';
  };
in {
  environment.systemPackages = [ display ];
  services.xserver.displayManager.sessionCommands = lib.mkBefore ''
    ${display}/bin/anima-display --auto || true
  '';
}
