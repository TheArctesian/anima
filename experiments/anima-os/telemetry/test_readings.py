"""Hardware files may disappear or omit capacity; never present unknown charge as zero."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import readings


class Telemetry(unittest.TestCase):
    def test_battery_hotplug_unknown_capacity_and_cpu_sample(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            def write(name, value):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(value)
            def path(name):
                return root / str(name).lstrip('/')
            write('sys/class/power_supply/BAT0/type', 'Battery')
            write('sys/class/power_supply/BAT0/capacity', '73')
            write('sys/class/power_supply/BAT0/status', 'Charging')
            write('sys/class/power_supply/mouse/type', 'Battery')
            write('sys/class/power_supply/mouse/scope', 'Device')
            write('proc/stat', 'cpu 10 0 10 80 0 0 0 0\n')
            write('proc/meminfo', 'MemTotal: 1000 kB\nMemAvailable: 750 kB\n')
            write('proc/uptime', '1234.0 0.0')
            with patch.object(readings, 'Path', path):
                reader = readings.Readings()
                first = reader.sample()
                self.assertIsNone(first['cpu'])
                self.assertEqual(first['memoryUsed'], 256000)
                self.assertEqual(first['batteries'], [{'name': 'BAT0', 'percent': 73, 'state': 'Charging'}])
                write('proc/stat', 'cpu 30 0 10 160 0 0 0 0\n')
                self.assertAlmostEqual(reader.sample()['cpu'], 20)
                (root/'sys/class/power_supply/BAT0/capacity').unlink()
                self.assertIsNone(reader.sample()['batteries'][0]['percent'])
                write('sys/class/power_supply/BAT0/present', '0')
                self.assertEqual(reader.sample()['batteries'], [])


if __name__ == '__main__':
    unittest.main()
