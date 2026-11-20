{ lib, pkgs, ... }:
let
  escape = builtins.fromJSON ''"\u001b"'';
  theme = pkgs.runCommand "anima-plymouth" {} ''
    mkdir -p $out/share/plymouth/themes/anima
    cp ${./branding/logo.png} $out/share/plymouth/themes/anima/logo.png
    cat > $out/share/plymouth/themes/anima/anima.plymouth <<THEME
    [Plymouth Theme]
    Name=Anima.ing
    Description=Anima.ing boot screen
    ModuleName=script
    [script]
    ImageDir=$out/share/plymouth/themes/anima
    ScriptFile=$out/share/plymouth/themes/anima/anima.script
    THEME
    cat > $out/share/plymouth/themes/anima/anima.script <<'SCRIPT'
    Window.SetBackgroundTopColor(0.180, 0.204, 0.251);
    Window.SetBackgroundBottomColor(0.180, 0.204, 0.251);
    logo.image = Image("logo.png").Scale(180, 180);
    logo.sprite = Sprite(logo.image);
    logo.sprite.SetX(Window.GetWidth() / 2 - 90);
    logo.sprite.SetY(Window.GetHeight() / 2 - 120);
    title.image = Image.Text("Anima.ing", 0.533, 0.753, 0.816);
    title.sprite = Sprite(title.image);
    title.sprite.SetX(Window.GetWidth() / 2 - title.image.GetWidth() / 2);
    title.sprite.SetY(Window.GetHeight() / 2 + 90);
    SCRIPT
  '';
in {
  system.nixos.distroId = "anima";
  system.nixos.distroName = "Anima.ing";
  system.nixos.label = "Preview";
  system.nixos.extraOSReleaseArgs = { PRETTY_NAME = "Anima.ing Preview"; HOME_URL = "https://anima.ing"; LOGO = "anima"; ANSI_COLOR = "38;2;136;192;208"; };
  boot.plymouth = { enable = true; theme = "anima"; themePackages = [ theme ]; logo = ./branding/logo.png; };
  boot.consoleLogLevel = 3;
  boot.kernelParams = [ "quiet" "splash" "plymouth.ignore-serial-consoles" "systemd.show_status=auto" "rd.systemd.show_status=auto" ];
  console.colors = [ "2e3440" "bf616a" "a3be8c" "ebcb8b" "81a1c1" "b48ead" "88c0d0" "e5e9f0" "4c566a" "bf616a" "a3be8c" "ebcb8b" "81a1c1" "b48ead" "8fbcbb" "eceff4" ];
  services.getty.helpLine = lib.mkForce "";
  environment.etc."issue".text = lib.mkForce ''
    ${escape}[36mAnima.ing${escape}[0m — server preview
    Network: nmtui   Status: anima-status   Enrollment: sudo anima-enroll

  '';
  programs.bash.promptInit = ''
    PS1='\[\e[36m\]anima.ing\[\e[0m\] \u@\h:\[\e[34m\]\w\[\e[0m\] \$ '
  '';
  hardware.enableRedistributableFirmware = true;
  environment.systemPackages = with pkgs; [ pciutils usbutils iw ethtool ];
}
