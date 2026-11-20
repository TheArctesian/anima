import json
import tempfile
from pathlib import Path
from unittest.mock import patch
import unittest
from orientation import outputs, main, save_rotation


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

    def test_confirmed_orientation_survives_and_overrides_panel_default(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def local_path(value):
                return root / str(value).lstrip('/')
            (root / 'persist').mkdir()
            display = 'eDP-1 connected 1920x1200+0+0 right (normal left inverted right)\n\tpanel orientation: Left Side Up\n'
            with patch('orientation.Path', side_effect=local_path), patch('pathlib.Path.is_mount', return_value=True), patch('orientation.os.geteuid', return_value=0), patch('orientation.subprocess.check_output', return_value=display):
                save_rotation('eDP-1')
                self.assertEqual(json.loads((root / 'persist/display.json').read_text()), {'eDP-1': 'right'})
                with patch('orientation.sys.argv', ['anima-display', '--auto']), patch('orientation.rotate') as rotate:
                    main()
                    rotate.assert_called_once_with('eDP-1', 'right')

    def test_saved_orientation_never_rotates_a_different_connector(self):
        with patch('orientation.saved_rotations', return_value={'eDP-1': 'right'}), patch('orientation.subprocess.check_output', return_value='HDMI-1 connected 1920x1080+0+0 (normal left inverted right)\n'), patch('orientation.sys.argv', ['anima-display', '--auto']), patch('orientation.rotate') as rotate:
            main()
            rotate.assert_not_called()
