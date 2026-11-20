{ pkgs, ... }:
{
  networking.hostName = "";
  system.name = "anima";
  environment.etc.hostname.source = "/persist/hostname";
  system.activationScripts.animaHostname = {
    deps = [ "etc" ];
    text = ''
      if [ ! -s /persist/hostname ]; then
        suffix=$(${pkgs.coreutils}/bin/od -An -N2 -tx1 /dev/urandom | ${pkgs.coreutils}/bin/tr -d ' \n')
        printf 'anima-node-%s\n' "$suffix" > /persist/hostname
        chmod 0644 /persist/hostname
      fi
    '';
  };
}
