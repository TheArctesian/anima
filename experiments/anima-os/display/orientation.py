"""Panel mounting correction and explicit rotation with timed rollback."""
import re
import subprocess
import sys

ROTATIONS = {'Normal': 'normal', 'Upside Down': 'inverted', 'Left Side Up': 'left', 'Right Side Up': 'right'}


def outputs(text):
    result = []
    for line in text.splitlines():
        match = re.match(r'^(\S+) connected(?: primary)? (\d+)x(\d+)\+(\d+)\+(\d+)(.*)', line)
        if match:
            tail = match[6].split('(')[0].strip().split()
            result.append({'name': match[1], 'rotation': tail[0] if tail and tail[0] in ROTATIONS.values() else 'normal', 'panel': None})
        elif line and not line[0].isspace():
            # Do not attach another connector's properties to the previous one.
            result.append({'name': None})
        elif result and 'panel orientation:' in line:
            result[-1]['panel'] = line.split(':', 1)[1].strip()
    return [entry for entry in result if entry['name']]


def rotate(output, rotation):
    subprocess.run(['xrandr', '--output', output, '--rotate', rotation], check=True)
    # Coordinate transforms for directly attached touchscreens on a single display.
    active = outputs(subprocess.check_output(['xrandr', '--prop'], text=True))
    if len(active) != 1:
        return
    matrices = {'normal': '1 0 0 0 1 0 0 0 1', 'left': '0 -1 1 1 0 0 0 0 1',
                'right': '0 1 0 -1 0 1 0 0 1', 'inverted': '-1 0 1 0 -1 1 0 0 1'}
    identifiers = subprocess.check_output(['xinput', '--list', '--id-only'], text=True).split()
    for identifier in identifiers:
        props = subprocess.check_output(['xinput', '--list-props', identifier], text=True)
        node = re.search(r'Device Node.*"([^\"]+)"', props)
        if not node:
            continue
        attrs = subprocess.check_output(['udevadm', 'info', '--query=property', '--name', node[1]], text=True)
        if 'ID_INPUT_TOUCHSCREEN=1' in attrs.splitlines():
            subprocess.run(['xinput', '--set-prop', identifier, 'Coordinate Transformation Matrix', *matrices[rotation].split()], check=True)


def dialog(args):
    return subprocess.run(['zenity', *args], capture_output=True, text=True)


def main():
    active = outputs(subprocess.check_output(['xrandr', '--prop'], text=True))
    if '--list' in sys.argv:
        for entry in active:
            print(entry['name'] + '\t' + entry['rotation'])
        return
    if '--set' in sys.argv:
        if len(sys.argv) != 4 or sys.argv[2] not in [d['name'] for d in active] or sys.argv[3] not in ROTATIONS.values():
            raise ValueError('Invalid display or rotation.')
        rotate(sys.argv[2], sys.argv[3])
        return
    if '--auto' in sys.argv:
        for entry in active:
            if entry['name'].startswith(('eDP', 'DSI', 'LVDS')) and entry['panel'] in ROTATIONS:
                rotate(entry['name'], ROTATIONS[entry['panel']])
        return
    choice = dialog(['--list', '--title=Display orientation', '--text=Choose a display', '--column=Display', *[d['name'] for d in active]])
    if choice.returncode != 0:
        return
    selected = next(d for d in active if d['name'] == choice.stdout.strip())
    choice = dialog(['--list', '--title=Display orientation', '--text=Choose a rotation. It reverts after 15 seconds unless you keep it.', '--column=Rotation', 'normal', 'left', 'right', 'inverted'])
    if choice.returncode != 0 or choice.stdout.strip() not in ROTATIONS.values():
        return
    try:
        rotate(selected['name'], choice.stdout.strip())
        keep = dialog(['--question', '--title=Keep this orientation?', '--text=Keep this orientation? Automatically reverting in 15 seconds.', '--timeout=15', '--ok-label=Keep', '--cancel-label=Revert'])
        if keep.returncode == 0:
            return
    except subprocess.CalledProcessError:
        dialog(['--error', '--text=Display adjustment failed. Restoring the previous orientation.'])
    rotate(selected['name'], selected['rotation'])


if __name__ == '__main__':
    main()
