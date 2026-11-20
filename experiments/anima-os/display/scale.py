"""Keep installer controls readable without shrinking the usable form viewport."""
import math
import re
import subprocess


def scale_for(width, height):
    # A quarter-step scale retains at least the 1280×800 logical form canvas.
    return max(1, min(3, math.floor(min(width / 1280, height / 800) * 4) / 4))


def main():
    try:
        text = subprocess.check_output(['xrandr', '--query'], text=True, timeout=5)
        screens = re.findall(r'^\S+ connected( primary)? (\d+)x(\d+)\+\d+\+\d+', text, re.M)
        if not screens:
            print(1)
            return
        primary = next((screen for screen in screens if screen[0]), screens[0])
        print(scale_for(int(primary[1]), int(primary[2])))
    except (OSError, subprocess.SubprocessError):
        print(1)


if __name__ == '__main__':
    main()
