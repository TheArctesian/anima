{ pkgs }:
pkgs.dockerTools.buildLayeredImage {
  name = "anima-preview-smoke";
  tag = "local";
  contents = [ pkgs.busybox ];
  extraCommands = ''
    mkdir -p www tmp
    chmod 1777 tmp
    echo anima-app-ok > www/index.html
  '';
  config.Cmd = [ "/bin/true" ];
}
