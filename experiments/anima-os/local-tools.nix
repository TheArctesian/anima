{ pkgs, daemonPackage }:
[
  (pkgs.writeShellApplication {
    name = "anima-enroll";
    runtimeInputs = [ pkgs.util-linux pkgs.systemd ];
    text = ''
      if [ "$(id -u)" -ne 0 ]; then
        echo "Run sudo anima-enroll from the installed machine." >&2
        exit 1
      fi
      echo "Create a device enrollment code in Anima. The hub must be reachable on your private network."
      read -r -p "Hub origin (for example https://your-private-hub): " hub
      runuser -u anima-daemon -- env ANIMA_DAEMON_HOME=/var/lib/anima-daemon \
        ${daemonPackage}/bin/anima-daemon enrol - --hub "$hub"
      systemctl restart anima-daemon
      echo "Enrollment saved. Check monitoring with anima-status."
    '';
  })
  (pkgs.writeShellApplication {
    name = "anima-status";
    runtimeInputs = [ pkgs.systemd pkgs.util-linux pkgs.networkmanager ];
    text = ''
      cat /etc/anima-preview
      echo ""
      echo "Network:"
      nmcli general status
      echo ""
      echo "Root mount:"
      findmnt -n -o TARGET,FSTYPE,OPTIONS /
      echo ""
      echo "Anima monitoring:"
      systemctl is-active anima-daemon || true
      echo "Enroll: sudo anima-enroll. Network setup: nmtui."
    '';
  })
]
