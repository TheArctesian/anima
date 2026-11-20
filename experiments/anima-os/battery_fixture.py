"""Synthetic laptop sysfs only inside disposable VMs; production readers stay unchanged."""
import time


def install(machine):
    machine.command("mkdir -p /run/battery-fixture/BAT0; printf 'Battery\\n' > /run/battery-fixture/BAT0/type; printf 'System\\n' > /run/battery-fixture/BAT0/scope; printf '1\\n' > /run/battery-fixture/BAT0/present; printf '73\\n' > /run/battery-fixture/BAT0/capacity; printf 'Charging\\n' > /run/battery-fixture/BAT0/status; mount --bind /run/battery-fixture /sys/class/power_supply")
    time.sleep(6)
