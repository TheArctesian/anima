"""Read actual rendered labels to drive the installer, without app test hooks."""
import csv
import io
import os
import subprocess
import time
from gui_test import screenshot, click


def locate(output, phrase, footer=False):
    screenshot(output, 'current')
    result = subprocess.run(['tesseract', str(output / 'current.ppm'), 'stdout', 'tsv'],
                            capture_output=True, text=True, check=True,
                            env={**os.environ, 'OMP_THREAD_LIMIT': '1'})
    height = int(subprocess.check_output(['magick', 'identify', '-format', '%h', str(output / 'current.ppm')], text=True)) if footer else 0
    lines = {}
    for word in csv.DictReader(io.StringIO(result.stdout), delimiter='\t'):
        if word['text'].strip() and (not footer or int(word['top']) > height-80):
            key = tuple(word[key] for key in ('block_num', 'par_num', 'line_num'))
            lines.setdefault(key, []).append(word)
    for words in lines.values():
        if phrase.lower() in ' '.join(word['text'] for word in words).lower():
            matches = [words[start:end] for start in range(len(words))
                       for end in range(start+1, len(words)+1)
                       if phrase.lower() in ' '.join(word['text'] for word in words[start:end]).lower()]
            words = min(matches, key=len)
            left = min(int(word['left']) for word in words)
            top = min(int(word['top']) for word in words)
            right = max(int(word['left'])+int(word['width']) for word in words)
            bottom = max(int(word['top'])+int(word['height']) for word in words)
            return left, top, right-left, bottom-top
    raise ValueError(f'Rendered text not found: {phrase}')


def tap(machine, output, phrase):
    if phrase in ('Continue', 'Erase selected disks', 'Shut down', 'Restart'):
        # Footer controls have stable placement; their dark-on-frost labels
        # are not reliably segmented by OCR. Subsequent state checks prove action.
        screenshot(output, 'current')
        dimensions = subprocess.check_output(['magick', 'identify', '-format', '%w %h', str(output/'current.ppm')], text=True)
        width, height = map(int, dimensions.split())
        x, y = width-(185 if phrase == 'Shut down' else 65), height-34
        if phrase in ('Shut down', 'Restart'):
            machine.command(f'(sleep 0.2; DISPLAY=:0 XAUTHORITY=/home/anima/.Xauthority xdotool mousemove {x} {y} click 1) </dev/null >/dev/null 2>&1 &')
            return
        click(machine, x, y)
    else:
        x, y, width, height = locate(output, phrase, footer=phrase in ('Shut down', 'Restart'))
        click(machine, x+width//2, y+height//2)
    time.sleep(0.5)
