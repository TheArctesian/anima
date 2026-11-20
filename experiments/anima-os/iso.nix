{ lib, pkgs, modulesPath, installedSystem, ... }:
let
  installerSource = builtins.path {
    path = ./installer;
    name = "anima-installer";
    filter = path: type:
      (type == "directory" && builtins.baseNameOf path != "__pycache__")
      || (type == "regular" && builtins.elem (builtins.baseNameOf path) [ "install.py" "disks.py" ]);
  };
  installer = pkgs.writeShellApplication {
    name = "anima-install";
    runtimeInputs = with pkgs; [ python3 util-linux parted btrfs-progs dosfstools systemd mkpasswd ];
    text = ''exec python3 ${installerSource}/install.py ${installedSystem}'';
  };
in {
  imports = [ "${modulesPath}/installer/cd-dvd/installation-cd-minimal.nix" ];
  networking.hostName = lib.mkForce "anima-live";
  networking.wireless.enable = lib.mkForce false;
  networking.networkmanager.enable = true;
  networking.useDHCP = lib.mkForce false;
  services.openssh.enable = lib.mkForce false;
  services.getty.autologinUser = lib.mkForce "root";
  boot.kernelParams = [ "console=tty0" "console=ttyS0,115200" ];
  environment.systemPackages = with pkgs; [ installer nano ];
  image.baseName = lib.mkForce "anima-os-preview-x86_64";
  image.fileName = lib.mkForce "anima-os-preview-x86_64.iso";
  boot.zfs.forceImportRoot = false;
  isoImage.volumeID = "ANIMA_PREVIEW";
  isoImage.squashfsCompression = "zstd -Xcompression-level 6";
  isoImage.storeContents = [ installedSystem ];
  environment.etc."issue".text = ''

    Anima OS — experimental x86-64 UEFI preview
    ==========================================
    Live trial: no installation occurs until you run anima-install.
    Network setup: nmtui
    Install: anima-install (selected disk erased after explicit confirmation)
    Shutdown: systemctl poweroff

    Local console is root in the live session; SSH is disabled.
    Installed system requires your chosen admin password.
    Managed jobs/agents, hub-driven setup and hardware support are unqualified.

  '';
  system.stateVersion = "26.05";
}
