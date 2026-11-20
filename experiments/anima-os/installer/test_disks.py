import copy
import unittest
import json
from unittest.mock import patch
import disks
from install import write_password
from pathlib import Path
import tempfile
import stat
from disks import eligible, validate


def disk():
    return {'path': '/dev/vda', 'maj:min': '252:0', 'type': 'disk', 'size': 64*1024**3,
            'ro': False, 'model': 'test', 'serial': 'one', 'mountpoints': [None]}


class DiskSafetyTests(unittest.TestCase):
    def test_password_file_is_private_and_cannot_overwrite_existing_state(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'password'
            write_password(path, 'test-hash')
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o600)
            self.assertEqual(path.read_text(), 'test-hash\n')
            with self.assertRaises(FileExistsError):
                write_password(path, 'replacement')
            self.assertEqual(path.read_text(), 'test-hash\n')

    def test_flat_partition_inventory_fails_closed(self):
        with patch('disks.subprocess.run') as run:
            run.return_value.stdout = json.dumps({'blockdevices': [disk(), {'type': 'part', 'path': '/dev/vda1'}]})
            with self.assertRaisesRegex(ValueError, 'dependency tree'):
                disks.inventory()

    def test_mounted_or_swap_child_is_never_eligible(self):
        for mount in ('/', '/iso', '[SWAP]'):
            candidate = disk()
            candidate['children'] = [{'type': 'part', 'mountpoints': [mount]}]
            self.assertFalse(eligible(candidate))

    def test_active_mapper_is_never_eligible_even_without_mount(self):
        candidate = disk()
        candidate['children'] = [{'type': 'part', 'children': [{'type': 'crypt', 'mountpoints': [None]}]}]
        self.assertFalse(eligible(candidate))

    def test_readonly_small_and_partition_targets_are_refused(self):
        for key, value in [('ro', True), ('size', 1024), ('type', 'part')]:
            candidate = disk(); candidate[key] = value
            self.assertFalse(eligible(candidate))

    def test_revalidation_rejects_changed_identity_or_new_mount(self):
        selected = disk()
        for key, value in [('serial', 'replacement'), ('size', 128*1024**3), ('mountpoints', ['/mnt'])]:
            current = copy.deepcopy(selected); current[key] = value
            with self.assertRaises(ValueError):
                validate(selected, [current])

    def test_matching_labels_on_another_disk_are_refused(self):
        other = disk(); other['path'] = '/dev/vdb'
        other['children'] = [{'type': 'part', 'label': 'ANIMA_OS'}]
        with self.assertRaisesRegex(ValueError, 'Another disk'):
            validate(disk(), [disk(), other])

    def test_nonblock_target_is_refused(self):
        candidate = disk(); candidate['path'] = '/dev/null'
        with self.assertRaisesRegex(ValueError, 'block device'):
            validate(candidate, [candidate])


if __name__ == '__main__':
    unittest.main()
