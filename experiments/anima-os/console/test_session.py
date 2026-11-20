"""Two local sessions must not race enrollment into the same daemon identity."""
import io
from pathlib import Path
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'display'))
from privileged import Session


class ConcurrentEnrollment(unittest.TestCase):
    def test_second_session_is_refused_until_first_releases_lease(self):
        entered, release = threading.Event(), threading.Event()
        first, second = Session(), Session()
        first.account.enroll = lambda: (entered.set(), release.wait(5))
        second.account.enroll = lambda: self.fail('Concurrent enrollment entered')
        with tempfile.TemporaryDirectory() as folder:
            def lock_file(*args):
                return io.open(Path(folder)/'lease', 'w')
            with patch('privileged.open', lock_file):
                worker = threading.Thread(target=lambda: first.dispatch('enroll', {}))
                worker.start()
                try:
                    self.assertTrue(entered.wait(2))
                    with self.assertRaisesRegex(ValueError, 'already running'):
                        second.dispatch('enroll', {})
                finally:
                    release.set()
                    worker.join(5)


if __name__ == '__main__':
    unittest.main()
