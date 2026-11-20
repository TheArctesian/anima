{ pkgs, lib, installedSystem, installerSource }:
let
  yaml = value: builtins.toJSON value;
  extension = pkgs.runCommand "anima-calamares" {} ''
    mkdir -p $out/{etc/calamares/modules,lib/calamares/modules/anima,share/calamares/branding/anima}
    cp ${./calamares/module.desc} $out/lib/calamares/modules/anima/module.desc
    cp ${./calamares/main.py} $out/lib/calamares/modules/anima/main.py
    substituteInPlace $out/lib/calamares/modules/anima/main.py \
      --replace-fail @installerSource@ ${installerSource} \
      --replace-fail @installedSystem@ ${installedSystem}
    cp ${./calamares/show.qml} $out/share/calamares/branding/anima/show.qml
    cp ${./calamares/branding.desc} $out/share/calamares/branding/anima/branding.desc
    cp ${./calamares/stylesheet.qss} $out/share/calamares/branding/anima/stylesheet.qss
    cp ${./branding/welcome.png} $out/share/calamares/branding/anima/welcome.png
    cp ${./branding/logo.png} $out/share/calamares/branding/anima/logo.png
    cat > $out/etc/calamares/settings.conf <<SETTINGS
    ${yaml {
      modules-search = [ "local" "@out@/lib/calamares/modules" ];
      branding = "anima";
      sequence = [ { show = [ "welcome" "keyboard" "users" "partition" "summary" ]; }
        { exec = [ "anima" ]; } { show = [ "finished" ]; } ];
      prompt-install = true; dont-chroot = false; oem-setup = false;
      disable-cancel = false; disable-cancel-during-exec = true;
      hide-back-and-next-during-exec = true; quit-at-end = false;
    }}
    SETTINGS
    substituteInPlace $out/etc/calamares/settings.conf --replace-fail @out@ $out
    cp ${pkgs.writeText "welcome.conf" (yaml {
      showSupportUrl = false; showKnownIssuesUrl = false; showReleaseNotesUrl = false;
      requirements = { requiredStorage = 32; requiredRam = 2; check = [ "storage" "ram" "root" ]; required = [ "storage" "ram" "root" ]; };
      geoip.style = "none";
    })} $out/etc/calamares/modules/welcome.conf
    cp ${pkgs.writeText "users.conf" (yaml {
      setRootPassword = false; doAutologin = false; allowWeakPasswords = false;
      passwordRequirements = { minLength = 12; maxLength = 128; };
      presets = { fullName = { value = "Anima owner"; editable = false; }; loginName = { value = "admin"; editable = false; }; };
      hostname = { location = "None"; writeHostsFile = false; };
    })} $out/etc/calamares/modules/users.conf
    cp ${pkgs.writeText "partition.conf" (yaml {
      efi = { mountPoint = "/boot"; recommendedSize = "1GiB"; minimumSize = "1GiB"; label = "ANIMA_EFI"; };
      userSwapChoices = [ "none" ]; defaultFileSystemType = "btrfs";
      availableFileSystemTypes = [ "btrfs" ]; allowManualPartitioning = false;
      enableLuksAutomatedPartitioning = false; lvm.enable = false;
      initialPartitioningChoice = "none"; defaultPartitionTableType = "gpt";
      partitionLayout = [ { name = "ANIMA_OS"; filesystem = "btrfs"; mountPoint = "/"; size = "100%"; } ];
    })} $out/etc/calamares/modules/partition.conf
    cp ${pkgs.writeText "finished.conf" (yaml { restartNowEnabled = false; restartNowChecked = false; })} $out/etc/calamares/modules/finished.conf
  '';
  calamares = pkgs.calamares.overrideAttrs (old: {
    patches = old.patches ++ [ ./calamares/selection.patch ./calamares/contrast.patch ];
    buildInputs = old.buildInputs ++ [ pkgs.python3 ];
  });
  launcher = pkgs.writeShellApplication {
    name = "anima-setup";
    runtimeInputs = with pkgs; [ python3 util-linux parted btrfs-progs dosfstools systemd mkpasswd ckbcomp ];
    text = ''
      if [[ $EUID -ne 0 ]]; then
        exec /run/wrappers/bin/sudo --preserve-env=DISPLAY,XAUTHORITY "$0"
      fi
      exec 9>/run/anima-installer.lock
      flock --nonblock 9 || exit 1
      umask 077
      python3 -c 'import sys,json; sys.path.insert(0,"${installerSource}"); from disks import inventory; print(json.dumps(inventory()))' > /run/anima-installer-disks.json
      export XDG_DATA_DIRS="${extension}/share:''${XDG_DATA_DIRS:-/run/current-system/sw/share}"
      export XDG_CONFIG_DIRS="${extension}/etc:/etc/xdg"
      exec ${calamares}/bin/calamares --xdg-config
    '';
  };
  desktop = pkgs.makeDesktopItem {
    name = "anima-setup"; desktopName = "Install Anima.ing";
    comment = "Install the Anima.ing server preview";
    exec = "${launcher}/bin/anima-setup"; icon = "${./branding/logo.png}";
    categories = [ "System" ];
  };
in { inherit launcher desktop calamares extension; }
