{ lib, pkgs, modulesPath, installedSystem, ... }:
let
  installerSource = builtins.path {
    path = ./installer;
    name = "anima-installer";
    filter = path: type:
      (type == "directory" && builtins.baseNameOf path != "__pycache__")
      || (type == "regular" && builtins.elem (builtins.baseNameOf path) [ "install.py" "disks.py" "volumes.py" ]);
  };
  installer = pkgs.writeShellApplication {
    name = "anima-install";
    runtimeInputs = with pkgs; [ python3 util-linux parted btrfs-progs dosfstools systemd mkpasswd ];
    text = ''
      exec 9>/run/anima-installer.lock
      flock --nonblock 9 || exit 1
      exec python3 ${installerSource}/install.py ${installedSystem}
    '';
  };
  gui = import ./calamares.nix { inherit pkgs lib installedSystem installerSource; };
in {
  imports = [ "${modulesPath}/installer/cd-dvd/installation-cd-graphical-base.nix" ./branding.nix ./live-desktop.nix ./display.nix ];
  networking.hostName = lib.mkForce "anima-live";
  # NetworkManager enables wpa_supplicant for its D-Bus Wi-Fi backend.
  # Forcing wireless off also removes that service and leaves adapters unavailable.
  networking.networkmanager.enable = true;
  networking.useDHCP = lib.mkForce false;
  services.openssh.enable = lib.mkForce false;
  services.getty.autologinUser = lib.mkForce "root";
  boot.kernelParams = [ "console=tty0" "console=ttyS0,115200" ];
  environment.systemPackages = with pkgs; [ installer nano gui.launcher gui.desktop gui.calamares networkmanagerapplet xdotool ];
  image.baseName = lib.mkForce "anima-os-preview-x86_64";
  image.fileName = lib.mkForce "anima-os-preview-x86_64.iso";
  boot.zfs.forceImportRoot = false;
  isoImage.grubTheme = pkgs.runCommand "anima-grub-theme" {} ''
    mkdir -p $out
    cp ${./branding/grub-theme.txt} $out/theme.txt
    cp ${pkgs.grub2}/share/grub/unicode.pf2 $out/unicode.pf2
  '';
  isoImage.efiSplashImage = ./branding/boot.png;
  isoImage.splashImage = ./branding/boot.png;
  isoImage.volumeID = "ANIMA_PREVIEW";
  isoImage.squashfsCompression = "zstd -Xcompression-level 6";
  isoImage.storeContents = [ installedSystem ];
  services.xserver.windowManager.openbox.enable = true;
  services.xserver.displayManager.lightdm.enable = true;
  services.displayManager.defaultSession = "none+openbox";
  programs.nm-applet.enable = false;
  programs.partition-manager.enable = true;
  users.users.nixos.name = "anima";
  services.displayManager.autoLogin = { enable = true; user = "anima"; };
  services.xserver.displayManager.sessionCommands = ''
    ( sleep 2; ${gui.launcher}/bin/anima-setup ) &
  '';
  system.activationScripts.animaDesktop = {
    deps = [ "users" "groups" ];
    text = ''
    mkdir -p /home/anima/Desktop /home/anima/.config/autostart
    cp ${gui.desktop}/share/applications/anima-setup.desktop /home/anima/Desktop/
    cp ${gui.desktop}/share/applications/anima-setup.desktop /home/anima/.config/autostart/
    chmod +x /home/anima/Desktop/anima-setup.desktop
    chown -R anima:users /home/anima/Desktop /home/anima/.config
  '';
  };
  system.stateVersion = "26.05";
}
