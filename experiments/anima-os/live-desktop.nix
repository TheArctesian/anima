{ pkgs, ... }:
{
  environment.systemPackages = [ pkgs.nordic pkgs.papirus-icon-theme ];
  services.xserver.desktopManager.xfce.enableScreensaver = false;
  services.logind.settings.Login = { HandleLidSwitch = "ignore"; HandleLidSwitchExternalPower = "ignore"; IdleAction = "ignore"; };
  systemd.targets.sleep.enable = false;
  systemd.targets.suspend.enable = false;
  systemd.targets.hibernate.enable = false;
  environment.etc."xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-power-manager.xml".text = ''
    <?xml version="1.0" encoding="UTF-8"?>
    <channel name="xfce4-power-manager" version="1.0"><property name="xfce4-power-manager" type="empty">
      <property name="dpms-enabled" type="bool" value="false"/>
      <property name="blank-on-ac" type="int" value="0"/>
      <property name="blank-on-battery" type="int" value="0"/>
      <property name="inactivity-on-ac" type="uint" value="14"/>
      <property name="inactivity-on-battery" type="uint" value="14"/>
    </property></channel>
  '';
  environment.etc."xdg/gtk-3.0/settings.ini".text = ''
    [Settings]
    gtk-theme-name=Nordic
    gtk-icon-theme-name=Papirus-Dark
    gtk-application-prefer-dark-theme=true
    gtk-font-name=DejaVu Sans 10
  '';
  environment.etc."xdg/xfce4/xfconf/xfce-perchannel-xml/xsettings.xml".text = ''
    <?xml version="1.0" encoding="UTF-8"?>
    <channel name="xsettings" version="1.0">
      <property name="Net" type="empty">
        <property name="ThemeName" type="string" value="Nordic"/>
        <property name="IconThemeName" type="string" value="Papirus-Dark"/>
      </property>
    </channel>
  '';
  environment.etc."xdg/xfce4/xfconf/xfce-perchannel-xml/xfwm4.xml".text = ''
    <?xml version="1.0" encoding="UTF-8"?>
    <channel name="xfwm4" version="1.0"><property name="general" type="empty">
      <property name="theme" type="string" value="Nordic"/>
    </property></channel>
  '';
  environment.etc."xdg/xfce4/terminal/terminalrc".text = ''
    [Configuration]
    ColorForeground=#ECEFF4
    ColorBackground=#2E3440
    ColorCursor=#88C0D0
    ColorPalette=#3B4252;#BF616A;#A3BE8C;#EBCB8B;#81A1C1;#B48EAD;#88C0D0;#E5E9F0;#4C566A;#BF616A;#A3BE8C;#EBCB8B;#81A1C1;#B48EAD;#8FBCBB;#ECEFF4
    ColorUseTheme=FALSE
    FontName=DejaVu Sans Mono 11
  '';
  services.xserver.displayManager.sessionCommands = ''
    ${pkgs.xset}/bin/xset s off -dpms
    # Monitor names are discovered in the live session, not assumed at build time.
    (
      ${pkgs.xrandr}/bin/xrandr --query | awk '/ connected/ {print $1}' | while read -r monitor; do
        for workspace in 0 1 2 3; do
          ${pkgs.xfconf}/bin/xfconf-query -c xfce4-desktop \
            -p "/backdrop/screen0/monitor$monitor/workspace$workspace/last-image" \
            --create -t string -s ${./branding/hero.png}
          ${pkgs.xfconf}/bin/xfconf-query -c xfce4-desktop \
            -p "/backdrop/screen0/monitor$monitor/workspace$workspace/image-style" \
            --create -t int -s 5
        done
      done
    ) &
  '';
}
