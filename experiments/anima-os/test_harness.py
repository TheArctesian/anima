"""The runner must not report success after detecting an unsafe test result."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import vm


class ResultTests(unittest.TestCase):
    def test_drive_option_syntax_in_a_path_is_refused_before_starting_qemu(self):
        with tempfile.TemporaryDirectory(prefix='anima,') as directory:
            output = Path(directory)
            with self.assertRaisesRegex(ValueError, 'commas'):
                vm.Machine(output, output / 'disk')
            self.assertFalse((output / 'serial.log').exists())

    def test_changed_unselected_disk_fails_the_run_and_keeps_evidence(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder)
            with patch.object(vm, 'prepare', return_value=(output / 'disk', 'before')), \
                 patch.object(vm, 'Machine'), patch.object(vm, 'digest', return_value='changed'):
                with self.assertRaisesRegex(RuntimeError, 'VM checks failed'):
                    vm.run(output / 'source', output)
            result = json.loads((output / 'results.json').read_text())[-1]
            self.assertEqual(result['case'], 'unselected-disk')
            self.assertEqual(result['result'], 'Failed')

    def test_zero_iterations_is_an_error_before_creating_output(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / 'unused'
            result = subprocess.run([
                sys.executable, str(Path(__file__).with_name('faults.py')),
                'missing.qcow2', str(output), '--scenario', 'shutdown-hang',
                '--iterations', '0',
            ], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('--iterations must be positive', result.stderr)
            self.assertFalse(output.exists())


if __name__ == '__main__':
    unittest.main()
