"""Synthetic contract tests for automatic enrollment; no production account calls."""
import unittest
from unittest.mock import patch
import hosted
from commands import fields


class Enrollment(unittest.TestCase):
    def setUp(self):
        self.account = hosted.Account()
        self.account.access = 'account-secret'
        self.commands = []
        self.queries = []

    def graph(self, query, variables=None, token=None):
        self.queries.append((query, variables, token))
        if 'claimDeviceToken' in query:
            return {'claimDeviceToken': {'token': 'hub-secret'}}
        if 'linkDevices' in query:
            return {'linkDevices': [{'name': 'anima-123456789abc', 'address': 'fd41:4e49:4d41::2'}]}
        if 'sites{' in query:
            return {'sites': [{'id': 'site', 'subnet': 'fd41:4e49:4d41::/48'}]}
        if 'devices(' in query:
            if variables['after'] is None:
                return {'devices': {'edges': [], 'pageInfo': {'hasNextPage': True, 'endCursor': 'next'}}}
            return {'devices': {'edges': [{'node': {'id': 'existing-device', 'host': 'fd41:4e49:4d41:0:0:0:0:2'}}], 'pageInfo': {'hasNextPage': False}}}
        if 'enrolDaemon' in query:
            self.assertEqual(variables, {'id': 'existing-device'})
            return {'enrolDaemon': {'code': 'one-time-secret'}}
        if 'logout' in query:
            return {'logout': True}
        self.fail('Unexpected mutation: '+query)

    def command(self, args, secret=None, timeout=25):
        self.commands.append((args, secret))
        self.assertFalse(any('secret' in arg for arg in args))
        return ''

    def test_automatic_enrollment_reuses_paginated_device_and_keeps_secrets_off_argv(self):
        with patch.object(hosted.Path, 'exists', lambda p: str(p) == hosted.SOCKET), \
             patch.object(hosted.Path, 'read_text', return_value='123456789abcdef'), \
             patch.object(hosted.time, 'sleep'), \
             patch.object(hosted, 'request', side_effect=[{'state': 'making'}, {'state': 'waking'}, {'state': 'ready'}, {'invite': 'invite-secret', 'claim': 'claim-secret'}]), \
             patch.object(hosted, 'graph', self.graph), patch.object(hosted, 'run', self.command):
            self.assertEqual(self.account.enroll(), {'enrolled': True})
        self.assertEqual(self.commands[0][1], 'invite-secret\n')
        self.assertEqual(self.commands[-2][1], 'one-time-secret\n')
        self.assertEqual(self.commands[-1][0], ['systemctl', 'restart', 'anima-daemon'])
        self.assertIsNone(self.account.access)

    def test_browser_confirmation_exposes_only_the_public_code(self):
        with patch.object(hosted, 'request', return_value={
            'device_code': 'private-polling-secret', 'user_code': 'ABCD-EFGH',
            'verification_uri_complete': 'https://auth.example/device?user_code=ABCD-EFGH',
            'interval': 5, 'expires_in': 300,
        }):
            result = self.account.start()
        self.assertEqual(result['userCode'], 'ABCD-EFGH')
        self.assertNotIn('private-polling-secret', str(result))

    def test_existing_owner_is_never_replaced_or_contacted(self):
        with patch.object(hosted.Path, 'exists', return_value=True), patch.object(hosted, 'request') as remote:
            with self.assertRaisesRegex(ValueError, 'already enrolled'):
                self.account.enroll()
            remote.assert_not_called()

    def test_failed_enrollment_does_not_start_monitoring_and_revokes_temporary_hub_token(self):
        def command(args, secret=None, timeout=25):
            if 'enrol' in args:
                raise ValueError('Refused')
            return self.command(args, secret, timeout)
        with patch.object(hosted.Path, 'exists', lambda p: str(p) == hosted.SOCKET), \
             patch.object(hosted.Path, 'read_text', return_value='123456789abcdef'), \
             patch.object(hosted, 'request', side_effect=[{'state': 'ready'}, {'invite': 'invite-secret', 'claim': 'claim-secret'}]), \
             patch.object(hosted, 'graph', self.graph), patch.object(hosted, 'run', command):
            with self.assertRaisesRegex(ValueError, 'Refused'):
                self.account.enroll()
        self.assertNotIn((['systemctl', 'restart', 'anima-daemon'], None), self.commands)
        self.assertIn(('mutation{logout}', None, 'hub-secret'), self.queries)

    def test_nmcli_keeps_adapter_and_escaped_ssid_identity(self):
        self.assertEqual(fields(r'wlp1s0:AA\:BB\:CC\:DD\:EE\:FF:Home\: Office:80:WPA2'),
                         ['wlp1s0', 'AA:BB:CC:DD:EE:FF', 'Home: Office', '80', 'WPA2'])


if __name__ == '__main__':
    unittest.main()
