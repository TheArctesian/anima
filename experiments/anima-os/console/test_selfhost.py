"""Manual enrollment preserves existing identity and keeps enrollment secrets off argv."""
from pathlib import Path
import unittest
from unittest.mock import patch
import selfhost


class SelfHosted(unittest.TestCase):
    def test_existing_ownership_and_invalid_origins_never_invoke_enrollment(self):
        with patch.object(Path, 'exists', return_value=True), patch.object(selfhost, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'already enrolled'):
                selfhost.enroll({'hub':'https://hub.example','code':'anima_enrol_AAAAAAAAAAAAAAAAAAAAAA'})
            run.assert_not_called()
        with patch.object(Path, 'exists', return_value=False), patch.object(selfhost, 'run') as run:
            for hub in ('http://hub.example', 'https://user@hub.example', 'https://hub.example:8443', 'https://hub.example/path'):
                with self.subTest(hub=hub), self.assertRaises(ValueError):
                    selfhost.enroll({'hub':hub,'code':'anima_enrol_AAAAAAAAAAAAAAAAAAAAAA'})
            run.assert_not_called()

    def test_success_hands_secret_to_stdin_and_starts_monitoring_only_after_enrollment(self):
        secret='anima_enrol_AAAAAAAAAAAAAAAAAAAAAA'
        sent=[]
        def execute(args, stdin=None, timeout=25):
            self.assertNotIn(secret,' '.join(args))
            sent.append((args,stdin))
        with patch.object(Path,'exists',return_value=False),patch.object(selfhost,'run',execute):
            self.assertTrue(selfhost.enroll({'hub':'https://hub.example','code':secret})['enrolled'])
        self.assertEqual(sent[0][1],secret+'\n')
        self.assertEqual(sent[-1][0],['systemctl','restart','anima-daemon'])
        with patch.object(Path,'exists',return_value=False),patch.object(selfhost,'run',side_effect=ValueError('refused')) as run:
            with self.assertRaisesRegex(ValueError,'refused'):
                selfhost.enroll({'hub':'https://hub.example','code':secret})
            self.assertEqual(run.call_count,1)


if __name__=='__main__':
    unittest.main()
