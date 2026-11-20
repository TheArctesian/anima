import copy
import unittest
from unittest.mock import patch
from test_disks import disk
from disks import eligible
from volumes import validate_plan


class VolumeAuthorizationTests(unittest.TestCase):
    def test_usb_and_removable_drives_are_excluded(self):
        for field, value in [('tran', 'usb'), ('rm', True)]:
            candidate = disk(); candidate[field] = value
            self.assertFalse(eligible(candidate))

    def test_same_disk_cannot_be_os_and_data(self):
        with self.assertRaisesRegex(ValueError, 'multiple'):
            validate_plan(disk(), [disk()], [disk()])

    def test_every_selected_disk_is_validated_before_writes(self):
        system = disk(); data = copy.deepcopy(system); data['path'] = '/dev/vdb'
        with patch('volumes.validate', side_effect=['/dev/vda', ValueError('changed')]) as validate:
            with self.assertRaisesRegex(ValueError, 'changed'):
                validate_plan(system, [data], [system, data])
            self.assertEqual(validate.call_count, 2)

    def test_unselected_disk_is_never_added_to_plan(self):
        system = disk(); data = copy.deepcopy(system); data['path'] = '/dev/vdb'
        other = copy.deepcopy(system); other['path'] = '/dev/vdc'
        with patch('volumes.validate', side_effect=lambda item, devices, **kwargs: item['path']):
            self.assertEqual(validate_plan(system, [data], [system, data, other]), ['/dev/vda', '/dev/vdb'])
