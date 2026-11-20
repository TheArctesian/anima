{ config, lib, pkgs, ... }:
{
  options.anima.revision = lib.mkOption { type = lib.types.str; default = "A"; };
  config = {
    environment.etc."anima-revision".text = config.anima.revision;
    environment.systemPackages = lib.optional (config.anima.revision == "B") pkgs.hello;
    systemd.services.anima-custom-test = lib.mkIf (config.anima.revision == "B") {
      wantedBy = [ "multi-user.target" ];
      after = [ "anima-test-state.service" ];
      requires = [ "anima-test-state.service" ];
      serviceConfig = { Type = "oneshot"; RemainAfterExit = true; User = "workload"; };
      script = "echo custom-service-B > /persist/workload/service-output";
    };
    systemd.tmpfiles.rules = [
      "d /persist/workload 0750 workload workload -"
      "d /persist/identity 0700 root root -"
      "d /var/lib/anima-prototype 0700 root root -"
    ];
    systemd.services.anima-boot-failure = {
      serviceConfig.Type = "oneshot";
      path = [ pkgs.systemd ];
      script = if config.anima.revision == "B" then
        "systemctl --no-block reboot"
      else
        "echo ANIMA_RECOVERY_REQUIRED >/dev/ttyS0";
    };
    systemd.services.anima-test-state = {
      onFailure = [ "anima-boot-failure.service" ];
      wantedBy = [ "multi-user.target" ];
      after = [ "systemd-tmpfiles-setup.service" ];
      before = [ "anima-root-lock.service" ];
      serviceConfig.Type = "oneshot";
      script = ''
        test -f /persist/identity/sentinel || echo synthetic-device-001 > /persist/identity/sentinel
        test -f /persist/workload/sentinel || echo retained-work-001 > /persist/workload/sentinel
      '';
    };
    systemd.services.anima-root-lock = {
      onFailure = [ "anima-boot-failure.service" ];
      wantedBy = [ "multi-user.target" ];
      after = [ "anima-test-state.service" "systemd-tmpfiles-setup.service" ];
      requires = [ "anima-test-state.service" ];
      before = [ "anima-local-health.service" ];
      serviceConfig = { Type = "oneshot"; RemainAfterExit = true; };
      path = [ pkgs.util-linux ];
      script = "mount -o remount,bind,ro /";
    };
    systemd.services.anima-local-health = {
      onFailure = [ "anima-boot-failure.service" ];
      wantedBy = [ "multi-user.target" ];
      after = [ "anima-root-lock.service" ];
      requires = [ "anima-root-lock.service" ];
      path = [ pkgs.util-linux pkgs.systemd ];
      serviceConfig = { Type = "oneshot"; RemainAfterExit = true; TimeoutStartSec = "30s"; };
      script = ''
        if { test -e /persist/fail-health && test '${config.anima.revision}' = B; } || test -e /persist/fail-all-health; then
          echo ANIMA_HEALTH_FAILED >/dev/ttyS0
          exit 1
        fi
        findmnt -n -o OPTIONS / | tr ',' '\n' | grep -qx ro
        test -f /persist/identity/sentinel
        test -f /persist/workload/sentinel
        test -e /sys/class/net/lo
        for counter in /sys/firmware/efi/efivars/LoaderBootCountPath-*; do
          if test -e "$counter"; then
            ${pkgs.systemd}/lib/systemd/systemd-bless-boot good
          fi
        done
        echo 'ANIMA_READY revision=${config.anima.revision}' >/dev/ttyS0
      '';
    };
  };
}
