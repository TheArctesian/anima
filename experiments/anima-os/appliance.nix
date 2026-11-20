{ config, lib, pkgs, daemonPackage, ... }:
{
  imports = [ ./branding.nix ./console.nix ./display.nix ];
  system.stateVersion = "26.05";
  networking.hostName = "anima";
  boot.loader.systemd-boot.enable = true;
  boot.loader.systemd-boot.configurationLimit = 3;
  boot.loader.efi.canTouchEfiVariables = false;
  boot.initrd.systemd.enable = true;
  boot.initrd.availableKernelModules = [ "virtio_pci" "virtio_blk" "ahci" "nvme" "xhci_pci" "usb_storage" "sd_mod" ];
  boot.kernelParams = [ "console=tty0" "console=ttyS0,115200" "panic=10" ];
  boot.tmp.useTmpfs = true;
  fileSystems = {
    "/" = { device = "/dev/disk/by-label/ANIMA_OS"; fsType = "btrfs"; options = [ "subvol=@root" "compress=zstd" ]; };
    "/nix" = { device = "/dev/disk/by-label/ANIMA_OS"; fsType = "btrfs"; options = [ "subvol=@nix" "compress=zstd" ]; };
    "/var" = { device = "/dev/disk/by-label/ANIMA_OS"; fsType = "btrfs"; options = [ "subvol=@var" "compress=zstd" ]; };
    "/persist" = { device = "/dev/disk/by-label/ANIMA_OS"; fsType = "btrfs"; options = [ "subvol=@persist" "compress=zstd" ]; neededForBoot = true; };
    "/boot" = { device = "/dev/disk/by-label/ANIMA_EFI"; fsType = "vfat"; options = [ "umask=0077" ]; };
    "/home" = { device = "/persist/home"; fsType = "none"; options = [ "bind" ]; depends = [ "/persist" ]; neededForBoot = true; };
    "/etc/NetworkManager/system-connections" = { device = "/persist/network-connections"; fsType = "none"; options = [ "bind" ]; depends = [ "/persist" ]; neededForBoot = true; };
  };
  networking.networkmanager.enable = true;
  networking.networkmanager.dns = "systemd-resolved";
  services.resolved.enable = true;
  networking.firewall.enable = true;
  services.openssh.enable = false;
  users.mutableUsers = false;
  users.users.root.hashedPassword = "!";
  users.users.admin = { isNormalUser = true; extraGroups = [ "wheel" "networkmanager" ]; hashedPasswordFile = "/persist/owner-password"; };
  users.users.anima-daemon = { isSystemUser = true; group = "anima-daemon"; };
  users.groups.anima-daemon = {};
  systemd.tmpfiles.rules = [ "d /var/lib/anima-link 0700 anima-daemon anima-daemon -" "d /var/lib/anima-daemon 0700 anima-daemon anima-daemon -" "d /persist/apps 0750 root root -" ];
  environment.systemPackages = [ (import ./link-package.nix { inherit pkgs; }) ] ++ (with pkgs; [ daemonPackage btrfs-progs curl jq git vim nano tmux podman-compose ])
    ++ import ./local-tools.nix { inherit pkgs daemonPackage; };
  environment.etc."anima-smoke-image.tar.gz".source = import ./smoke-image.nix { inherit pkgs; };
  virtualisation.podman.enable = true;
  virtualisation.podman.dockerCompat = true;
  services.logind.settings.Login = { HandleLidSwitch = "ignore"; HandleLidSwitchExternalPower = "ignore"; IdleAction = "ignore"; };
  systemd.targets.sleep.enable = false;
  systemd.targets.suspend.enable = false;
  systemd.targets.hibernate.enable = false;
  systemd.services.anima-data-volumes = {
    wantedBy = [ "multi-user.target" ];
    after = [ "local-fs.target" ];
    path = [ pkgs.util-linux ];
    serviceConfig.Type = "oneshot";
    serviceConfig.RemainAfterExit = true;
    serviceConfig.ExecStart = "${pkgs.python3}/bin/python3 ${./installer/mount_volumes.py}";
  };
  systemd.services.anima-keymap = {
    wantedBy = [ "multi-user.target" ];
    after = [ "local-fs.target" "systemd-vconsole-setup.service" ];
    before = [ "getty.target" ];
    unitConfig.ConditionPathExists = "/persist/console.map";
    serviceConfig.Type = "oneshot";
    path = [ pkgs.kbd pkgs.coreutils ];
    script = "loadkeys /persist/console.map";
  };
  systemd.services.anima-root-lock = {
    wantedBy = [ "multi-user.target" ];
    after = [ "systemd-tmpfiles-setup.service" ];
    before = [ "anima-daemon.service" ];
    serviceConfig = { Type = "oneshot"; RemainAfterExit = true; TimeoutStartSec = "30s"; };
    path = [ pkgs.util-linux ];
    script = "mount -o remount,bind,ro /";
  };
  systemd.services.anima-daemon = {
    description = "Anima enrolled device monitoring";
    wantedBy = [ "multi-user.target" ];
    after = [ "network-online.target" "anima-root-lock.service" ];
    wants = [ "network-online.target" ];
    requires = [ "anima-root-lock.service" ];
    unitConfig.ConditionPathExists = "/var/lib/anima-daemon/daemon.json";
    environment.ANIMA_DAEMON_HOME = "/var/lib/anima-daemon";
    path = [ pkgs.coreutils pkgs.util-linux ];
    serviceConfig = {
      ExecStart = "${daemonPackage}/bin/anima-daemon run";
      User = "anima-daemon"; Group = "anima-daemon";
      Restart = "on-failure"; RestartSec = 10;
      StateDirectory = "anima-daemon"; StateDirectoryMode = "0700"; UMask = "0077";
      NoNewPrivileges = true; CapabilityBoundingSet = ""; AmbientCapabilities = "";
      ProtectSystem = "strict"; ProtectHome = true; PrivateTmp = true;
      ProtectKernelTunables = true; ProtectKernelModules = true; ProtectKernelLogs = true;
      ProtectControlGroups = true; ProtectClock = true; ProtectHostname = true;
      LockPersonality = true; RestrictRealtime = true; RestrictSUIDSGID = true;
      RestrictNamespaces = true; RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_UNIX" ];
      SystemCallArchitectures = "native"; SystemCallFilter = [ "@system-service" ];
      SystemCallErrorNumber = "EPERM"; MemoryDenyWriteExecute = true;
      ProtectProc = "invisible"; PrivateIPC = true; RemoveIPC = true; LimitCORE = 0;
      SocketBindDeny = "any";
      IPAddressDeny = "any";
      IPAddressAllow = [ "localhost" "100.64.0.0/10" "fd7a:115c:a1e0::/48" ];
    };
  };
  systemd.services.anima-link = {
    description = "Anima persistent private hub link";
    wantedBy = [ "multi-user.target" ];
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    before = [ "anima-daemon.service" ];
    unitConfig.ConditionPathExists = "/var/lib/anima-link/device.json";
    serviceConfig = {
      User = "anima-daemon"; Group = "anima-daemon";
      ExecStart = "${import ./link-package.nix { inherit pkgs; }}/bin/anima-link device -state /var/lib/anima-link -daemon-socket /var/lib/anima-daemon/link.sock";
      Restart = "on-failure"; RestartSec = 5; UMask = "0077";
      NoNewPrivileges = true; ProtectSystem = "strict"; ProtectHome = true;
      ReadWritePaths = [ "/var/lib/anima-link" "/var/lib/anima-daemon" ];
      PrivateTmp = true; LimitCORE = 0;
    };
  };
  services.tailscale.enable = true;
  services.journald.extraConfig = "SystemMaxUse=128M";
  systemd.targets.reboot.unitConfig = { JobTimeoutSec = "60s"; JobTimeoutAction = "reboot-force"; };
  systemd.settings.Manager = { RuntimeWatchdogSec = "20s"; RebootWatchdogSec = "60s"; };
  environment.etc."anima-preview".text = "Experimental UEFI preview. Enrollment/monitoring and local Podman only; managed jobs/agents are not integrated.\n";
}
