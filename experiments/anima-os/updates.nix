{ pkgs, ... }:
let
  source = pkgs.runCommand "anima-update-source" {} ''
    mkdir -p $out
    cp ${./updates/update.py} $out/update.py
    cp ${./updates/feed.py} $out/feed.py
    cp ${./updates/boot.py} $out/boot.py
    cp ${./updates/preview-public.pem} $out/preview-public.pem
    substituteInPlace $out/update.py --replace-fail @blessBoot@ ${pkgs.systemd}/lib/systemd/systemd-bless-boot
  '';
  updater = pkgs.writeShellApplication {
    name = "anima-os-update";
    runtimeInputs = with pkgs; [ python3 openssl nix systemd util-linux coreutils ];
    text = ''exec python3 ${source}/update.py "$@"'';
  };
in {
  environment.systemPackages = [ updater ];
  systemd.services.systemd-bless-boot.enable = false;
  systemd.services.anima-update-health = {
    wantedBy = [ "multi-user.target" ];
    after = [ "anima-root-lock.service" "NetworkManager.service" "wpa_supplicant.service" "display-manager.service" "anima-daemon.service" ];
    onFailure = [ "anima-update-failure.service" ];
    serviceConfig = { Type = "oneshot"; TimeoutStartSec = "90s"; ExecStart = "${updater}/bin/anima-os-update health"; };
  };
  systemd.services.anima-update-failure = {
    serviceConfig.Type = "oneshot";
    script = ''${updater}/bin/anima-os-update failure'';
  };
}
