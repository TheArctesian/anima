import unittest
from orientation import outputs


class OrientationTests(unittest.TestCase):
    def test_only_active_outputs_and_their_own_orientation(self):
        text = '''Screen 0: minimum 8 x 8, current 800 x 1280, maximum 32767 x 32767
eDP-1 connected primary 800x1280+0+0 left (normal left inverted right x axis y axis)
\tpanel orientation: Left Side Up
HDMI-1 disconnected (normal left inverted right x axis y axis)
\tpanel orientation: Upside Down
DP-1 connected 1920x1080+800+0 (normal left inverted right x axis y axis)
'''
        self.assertEqual(outputs(text), [
            {'name': 'eDP-1', 'rotation': 'left', 'panel': 'Left Side Up'},
            {'name': 'DP-1', 'rotation': 'normal', 'panel': None}])
