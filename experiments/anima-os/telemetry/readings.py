"""Local Linux readings shared by the installer and console; missing values stay unknown."""
import json
from pathlib import Path


def text(path):
    try:
        return path.read_text().strip()
    except OSError:
        return ''


def number(path):
    try:
        return float(text(path))
    except ValueError:
        return None


def batteries():
    result = []
    for device in sorted(Path('/sys/class/power_supply').glob('*')):
        if text(device / 'type') != 'Battery' or text(device / 'scope') == 'Device' or text(device / 'present') == '0':
            continue
        percent = number(device / 'capacity')
        result.append({'name': device.name, 'percent': percent if percent is not None and 0 <= percent <= 100 else None,
                       'state': text(device / 'status') or 'Unknown'})
    return result


class Readings:
    def __init__(self):
        self.previous = None

    def sample(self):
        cpu = None
        try:
            fields = [int(value) for value in text(Path('/proc/stat')).splitlines()[0].split()[1:9]]
            current = (sum(fields), fields[3] + fields[4])
            if self.previous and current[0] > self.previous[0]:
                cpu = max(0, min(100, 100 * (1 - (current[1] - self.previous[1]) / (current[0] - self.previous[0]))))
            self.previous = current
        except (ValueError, IndexError):
            self.previous = None
        memory = {}
        for line in text(Path('/proc/meminfo')).splitlines():
            fields = line.split()
            if fields[0] in ('MemTotal:', 'MemAvailable:'):
                memory[fields[0][:-1]] = int(fields[1]) * 1024
        total, available = memory.get('MemTotal'), memory.get('MemAvailable')
        return {'cpu': cpu, 'memoryTotal': total,
                'memoryUsed': total - available if total is not None and available is not None else None,
                'uptime': text(Path('/proc/uptime')).split(' ')[0] or None, 'batteries': batteries()}


if __name__ == '__main__':
    print(json.dumps(batteries()))
