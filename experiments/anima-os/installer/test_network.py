"""Network-copy regression checks use synthetic profiles only."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from install import copy_network_profiles


class NetworkCopyTests(unittest.TestCase):
    def test_profiles_are_private_and_symlinks_are_not_followed(self):
        with tempfile.TemporaryDirectory() as folder:
            source = Path(folder) / 'source'; source.mkdir()
            target = Path(folder) / 'target'; target.mkdir()
            profile = source / 'wifi.nmconnection'
            profile.write_text('[connection]\nid=test-fixture\n')
            (source / 'link.nmconnection').symlink_to(profile)
            # Test runner is unprivileged; only substitute the ownership check.
            original = Path.stat
            def synthetic_root(path, **kwargs):
                result = original(path, **kwargs)
                return type('Metadata', (), {'st_uid': 0, 'st_mode': result.st_mode})()
            with patch.object(Path, 'stat', synthetic_root):
                copy_network_profiles(target, source)
            self.assertEqual((target / profile.name).read_text(), profile.read_text())
            self.assertEqual((target / profile.name).stat().st_mode & 0o777, 0o600)
            self.assertFalse((target / 'link.nmconnection').exists())


if __name__ == '__main__':
    unittest.main()
