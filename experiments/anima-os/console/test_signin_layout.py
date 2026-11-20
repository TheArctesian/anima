"""Exercise the actual sign-in window without contacting an account provider."""
import argparse
import importlib.util
from pathlib import Path
import sys
from PySide6.QtCore import Qt
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication, QPushButton
from PySide6.QtWebEngineWidgets import QWebEngineView


def check_windows(output):
    back = next(widget for widget in QApplication.allWidgets()
                if isinstance(widget, QPushButton) and widget.text() == '← Back to Anima')
    browser = back.window()
    web = browser.findChild(QWebEngineView)
    assert web.page().webChannel() is None, 'Remote sign-in must not receive the native bridge'
    assert web.page().profile().isOffTheRecord(), 'Sign-in must not keep a persistent browser profile'
    browser.show()
    for width, height in ((1280, 800), (1920, 1200), (800, 1280), (640, 480)):
        browser.resize(width, height)
        QTest.qWait(150)
        browser.grab().save(str(output / f'signin-{width}x{height}.png'))
        print(f'{width}x{height}: page {web.geometry().getRect()}', flush=True)
        assert web.y() <= 96, 'Sign-in header consumes too much vertical space'
        assert web.height() >= height - 96, 'Sign-in form does not fill remaining screen space'
        assert web.width() >= width - 32, 'Sign-in form is unnecessarily narrow'
        assert browser.rect().contains(back.geometry()) or browser.rect().contains(back.mapTo(browser, back.rect().center()))
    QTest.mouseClick(back, Qt.MouseButton.LeftButton)
    assert not browser.isVisible(), 'Back must return to the local console'
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, default=Path(__file__).with_name('app.py'))
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    spec = importlib.util.spec_from_file_location('console_under_test', args.source)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    QApplication.exec = staticmethod(lambda: check_windows(args.output))
    return module.main()


if __name__ == '__main__':
    sys.exit(main())
