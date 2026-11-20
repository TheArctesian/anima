"""Keep the real installer settings and its Done action on-screen after rotation."""
import argparse
import json
from pathlib import Path
import time
from gui_test import click, prepare, screenshot
from vm import Machine, digest


def check_panel(machine):
    machine.command('eval "$(xdotool getwindowgeometry --shell "$panel")"; '
                    'read screen_width screen_height <<< "$(xdotool getdisplaygeometry)"; '
                    'printf "panel %s %s %s %s screen %s %s\\n" "$X" "$Y" "$WIDTH" "$HEIGHT" "$screen_width" "$screen_height"; '
                    'test "$X" -ge 0; test "$Y" -ge 0; '
                    'test "$((X+WIDTH))" -le "$screen_width"; '
                    'test "$((Y+HEIGHT))" -le "$screen_height"; '
                    'test "$((2*X+WIDTH-screen_width))" -ge -2; '
                    'test "$((2*X+WIDTH-screen_width))" -le 2; '
                    'test "$((2*Y+HEIGHT-screen_height))" -ge -2; '
                    'test "$((2*Y+HEIGHT-screen_height))" -le 2')


def rotation_panel(machine, output):
    machine.expect(r'root@anima-live:[^\r\n]*#', timeout=240)
    machine.send('stty -echo; export DISPLAY=:0 XAUTHORITY=/home/anima/.Xauthority')
    machine.command("timeout 90 sh -c 'until xdotool search --onlyvisible --class calamares; do sleep 1; done'", timeout=100)
    time.sleep(3)
    machine.send('main=$(xdotool getactivewindow)')
    machine.command('xrandr --output Virtual-1 --rotate left')
    time.sleep(2)
    click(machine, 345, 1246)
    machine.send('panel=$(xdotool getactivewindow)')
    machine.command('test "$panel" != "$main"')
    machine.command('xrandr --output Virtual-1 --rotate normal')
    time.sleep(2)
    screenshot(output, 'landscape-panel')
    check_panel(machine)
    for mode in ('800x600', '640x480'):
        machine.command(f'xrandr --output Virtual-1 --mode {mode}')
        time.sleep(2)
        screenshot(output, f'panel-{mode}')
        check_panel(machine)
    machine.command('eval "$(xdotool getwindowgeometry --shell "$panel")"; '
                    'xdotool mousemove "$((X+WIDTH/2))" "$((Y+HEIGHT-38))" click 1')
    time.sleep(1)
    machine.command('test "$(xdotool getactivewindow)" != "$panel"')
    screenshot(output, 'done-closed-panel')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--iso', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    disk = prepare(args.output)
    result = {'iso_sha256': digest(args.iso), 'case': 'rotation-panel-bounds-and-done'}
    machine = Machine(args.output, disk, args.iso)
    try:
        rotation_panel(machine, args.output)
        result['result'] = 'Passed'
    except Exception as error:
        result.update(result='Failed', error=str(error))
        raise
    finally:
        machine.close()
        (args.output/'results.json').write_text(json.dumps(result, indent=2)+'\n')


if __name__ == '__main__':
    main()
