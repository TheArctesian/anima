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
    cp ${./branding/back.svg} $out/share/calamares/branding/anima/back.svg
    cp ${./branding/next.svg} $out/share/calamares/branding/anima/next.svg
    cp ${./branding/logo.png} $out/share/calamares/branding/anima/logo.png
    cat > $out/etc/calamares/settings.conf <<SETTINGS
    ${yaml {
      modules-search = [ "local" "@out@/lib/calamares/modules" ];
      branding = "anima";
      instances = map (name: { module = "animasetup"; id = name; config = "${name}.conf"; }) [ "Welcome" "Network" "Storage" "Review" "Ready" ];
      sequence = [ { show = [ "animasetup@Welcome" "animasetup@Network" "keyboard" "users" "animasetup@Storage" "animasetup@Review" ]; }
        { exec = [ "anima" ]; } { show = [ "animasetup@Ready" ]; } ];
      prompt-install = false; dont-chroot = false; oem-setup = false;
      disable-cancel = true; disable-cancel-during-exec = true;
      hide-back-and-next-during-exec = true; quit-at-end = false;
    }}
    SETTINGS
    substituteInPlace $out/etc/calamares/settings.conf --replace-fail @out@ $out
    for page in Welcome Network Storage Review Ready; do
      echo "page: $page" > $out/etc/calamares/modules/$page.conf
    done
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
    patches = old.patches ++ [ ./calamares/selection.patch ./calamares/contrast.patch ./calamares/fullscreen.patch ./calamares/forms.patch ];
    postPatch = (old.postPatch or "") + ''
      cp -r ${./calamares/animasetup} src/modules/animasetup
      cp ${./calamares/shell/AnimaSettings.h} src/calamares/AnimaSettings.h
      chmod -R u+w src/modules/animasetup
      substituteInPlace src/modules/animasetup/Hero.cpp --replace-fail @hero@ ${./branding/hero.png}
      substituteInPlace src/calamares/CalamaresWindow.cpp --replace-fail @wordmark@ ${./branding/wordmark.svg} --replace-fail @settingsIcon@ ${./branding/settings.svg} --replace-fail @backIcon@ ${./branding/back.svg} --replace-fail @nextIcon@ ${./branding/next.svg}
    '';
    buildInputs = old.buildInputs ++ [ pkgs.python3 ];
  });
  launcher = pkgs.writeShellApplication {
    name = "anima-setup";
    runtimeInputs = with pkgs; [ python3 util-linux parted btrfs-progs dosfstools systemd mkpasswd ckbcomp networkmanager networkmanagerapplet xfce4-terminal xrandr ];
    text = ''
      if [[ $EUID -ne 0 ]]; then
        exec /run/wrappers/bin/sudo --preserve-env=DISPLAY,XAUTHORITY "$0"
      fi
      exec 9>/run/anima-installer.lock
      flock --nonblock 9 || exit 1
      umask 077
      python3 -c 'import sys,json; sys.path.insert(0,"${installerSource}"); from disks import inventory,eligible; print(json.dumps([dict(d,eligible=eligible(d)) for d in inventory()]))' > /run/anima-installer-disks.json
      export XDG_DATA_DIRS="${extension}/share:''${XDG_DATA_DIRS:-/run/current-system/sw/share}"
      export XDG_CONFIG_DIRS="${extension}/etc:/etc/xdg"
      export QT_FONT_DPI=96
      QT_SCALE_FACTOR=$(python3 ${./display/scale.py})
      export QT_SCALE_FACTOR
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
