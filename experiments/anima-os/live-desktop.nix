{ pkgs, ... }:
{
  environment.systemPackages = [ pkgs.nordic pkgs.papirus-icon-theme pkgs.xfce4-terminal ];
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
    gtk-font-name=IBM Plex Sans 10
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
  environment.etc."xdg/openbox/rc.xml".text = ''
    <openbox_config xmlns="http://openbox.org/3.4/rc">
      <applications><application class="*calamares*"><decor>no</decor></application></applications>
      <keyboard><keybind key="C-A-t"><action name="Execute"><command>xfce4-terminal --disable-server</command></action></keybind></keyboard>
    </openbox_config>
  '';
  fonts.packages = [ pkgs.ibm-plex ];
  services.xserver.displayManager.sessionCommands = ''
    ${pkgs.xset}/bin/xset s off -dpms
    ${pkgs.xsetroot}/bin/xsetroot -solid '#1B1D21'
  '';
}
