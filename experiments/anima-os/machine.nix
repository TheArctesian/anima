{ config, lib, pkgs, ... }:
{
  imports = [ ./runtime.nix ];
  system.stateVersion = "26.05";
  networking.hostName = "anima-prototype";
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = false;
  boot.initrd.availableKernelModules = [ "virtio_pci" "virtio_blk" "virtio_net" "ahci" ];
  boot.kernelParams = [ "console=ttyS0,115200" "panic=10" ];
  fileSystems = {
    "/" = { device = "/dev/disk/by-label/ANIMA"; fsType = "btrfs"; options = [ "subvol=@root" ]; };
    "/nix" = { device = "/dev/disk/by-label/ANIMA"; fsType = "btrfs"; options = [ "subvol=@nix" ]; };
    "/var" = { device = "/dev/disk/by-label/ANIMA"; fsType = "btrfs"; options = [ "subvol=@var" ]; };
    "/persist" = { device = "/dev/disk/by-label/ANIMA"; fsType = "btrfs"; options = [ "subvol=@persist" ]; };
    "/boot" = { device = "/dev/disk/by-label/ESP"; fsType = "vfat"; options = [ "umask=0077" ]; };
  };
  boot.tmp.useTmpfs = true;
  networking.useDHCP = false; # No NIC in the offline test fixture.
  networking.resolvconf.enable = false;
  services.resolved.enable = true;
  services.logrotate.enable = false; # Journald-only prototype; retention qualification is M5.
  services.journald.extraConfig = "SystemMaxUse=64M";
  networking.firewall.enable = true;
  services.openssh.enable = false;
  services.getty.autologinUser = "root"; # Disposable serial-only fixture, never a product default.
  users.users = {
    root.initialHashedPassword = "!";
    workload = { isSystemUser = true; group = "workload"; };
    daemon-test = { isSystemUser = true; group = "daemon-test"; };
  };
  users.groups.workload = {};
  users.groups.daemon-test = {};
  environment.systemPackages = with pkgs; [ btrfs-progs curl jq util-linux ];
  nix.settings.auto-optimise-store = false;
  systemd.services.systemd-bless-boot.enable = false;
  systemd.settings.Manager.RuntimeWatchdogSec = "20s";
  systemd.settings.Manager.RebootWatchdogSec = "60s";
}
