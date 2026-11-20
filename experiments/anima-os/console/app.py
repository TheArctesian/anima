"""Unprivileged fullscreen web UI. Only packaged local content receives the native bridge."""
import json
from pathlib import Path
import sys
from PySide6.QtCore import QObject, QProcess, Qt, QTimer, QUrl, Signal, Slot
from PySide6.QtGui import QColor
from PySide6.QtWidgets import QApplication, QHBoxLayout, QLabel, QMainWindow, QPushButton, QVBoxLayout, QWidget
from PySide6.QtWebChannel import QWebChannel
from PySide6.QtWebEngineCore import QWebEnginePage, QWebEngineProfile, QWebEngineSettings
from PySide6.QtWebEngineWidgets import QWebEngineView

ROOT = Path(__file__).resolve().parent


class LocalPage(QWebEnginePage):
    def acceptNavigationRequest(self, url, kind, main_frame):
        return url.isLocalFile() and Path(url.toLocalFile()).resolve() == ROOT / 'index.html'


class BrowserPage(QWebEnginePage):
    def acceptNavigationRequest(self, url, kind, main_frame):
        return url.scheme() == 'https' or url.toString() == 'about:blank'


class Bridge(QObject):
    answer = Signal(str)
    locked = Signal()
    openBrowser = Signal(str, str)
    closeBrowser = Signal()

    def __init__(self):
        super().__init__()
        self.process = None
        self.buffer = b''
        self.busy = False
        self.unlocked = False

    def emit_error(self, identity, message):
        self.answer.emit(json.dumps({'id': identity, 'ok': False, 'error': message}))

    @Slot(str)
    def request(self, text):
        try:
            request = json.loads(text)
            identity, op = request['id'], request['op']
            if op == 'lock':
                self.lock()
                return
            if op == 'unlock':
                if self.process:
                    self.emit_error(identity, 'Authentication is already in progress.')
                    return
                self.unlock(request['args']['password'])
                return
            if not self.unlocked:
                self.emit_error(identity, 'Unlock the local session first.')
                return
            if self.busy:
                self.emit_error(identity, 'Wait for the current operation to finish.')
                return
            self.busy = True
            self.process.write((json.dumps(request)+'\n').encode())
        except (ValueError, KeyError, TypeError):
            self.emit_error('invalid', 'Invalid local request.')

    def unlock(self, password):
        if not isinstance(password, str) or '\n' in password or '\r' in password:
            self.emit_error('unlock', 'Invalid password.')
            return
        self.buffer = b''
        process = QProcess(self)
        self.process = process
        process.readyReadStandardOutput.connect(lambda: self.read(process))
        process.finished.connect(lambda: self.ended(process))
        process.errorOccurred.connect(lambda: self.ended(process))
        process.start('/run/wrappers/bin/sudo', ['-k', '-S', '-p', '', '--preserve-env=DISPLAY,XAUTHORITY', 'anima-console-privileged'])
        process.write((password+'\n').encode())
        QTimer.singleShot(15000, lambda: self.auth_timeout(process))

    def auth_timeout(self, process):
        if process is self.process and not self.unlocked:
            self.emit_error('unlock', 'Authentication timed out. Try again.')
            self.lock()

    def read(self, process):
        if process is not self.process:
            process.readAllStandardOutput()
            return
        self.buffer += bytes(process.readAllStandardOutput())
        while b'\n' in self.buffer:
            line, self.buffer = self.buffer.split(b'\n', 1)
            try:
                answer = json.loads(line)
            except ValueError:
                continue
            self.busy = False
            if answer['id'] == 'unlock' and answer['ok']:
                self.unlocked = True
            value = answer.get('value', {})
            if isinstance(value, dict):
                if value.get('url'):
                    self.openBrowser.emit(value['url'], value.get('userCode', ''))
                if value.get('signedIn'):
                    self.closeBrowser.emit()
            self.answer.emit(json.dumps(answer))

    def ended(self, process):
        if process is not self.process:
            process.deleteLater()
            return
        self.process = None
        self.busy = False
        if not self.unlocked:
            self.emit_error('unlock', 'Authentication failed. Enter the local admin password.')
        self.unlocked = False
        self.locked.emit()
        process.deleteLater()

    @Slot()
    def lock(self):
        process = self.process
        self.process = None
        self.unlocked = False
        self.busy = False
        self.buffer = b''
        if process:
            process.closeWriteChannel()
        self.closeBrowser.emit()
        self.locked.emit()


def main():
    app = QApplication(sys.argv)
    app.setApplicationName('Anima local console')
    window = QMainWindow()
    window.setWindowFlags(Qt.WindowType.FramelessWindowHint)
    view = QWebEngineView(window)
    profile = QWebEngineProfile('AnimaConsole', view)
    page = LocalPage(profile, view)
    page.setBackgroundColor(QColor('#1B1D21'))
    page.settings().setAttribute(QWebEngineSettings.WebAttribute.LocalContentCanAccessRemoteUrls, False)
    page.settings().setAttribute(QWebEngineSettings.WebAttribute.LocalContentCanAccessFileUrls, True)
    view.setPage(page)
    view.setContextMenuPolicy(Qt.ContextMenuPolicy.NoContextMenu)
    bridge = Bridge()
    channel = QWebChannel(page)
    channel.registerObject('console', bridge)
    page.setWebChannel(channel)
    window.setCentralWidget(view)
    browser = QWidget()
    browser.setWindowFlags(Qt.WindowType.FramelessWindowHint)
    browser.setStyleSheet('QWidget{background:#1B1D21;color:#E6E7E9;font:14px "IBM Plex Sans";}QPushButton{padding:8px 14px;border:1px solid #353941;border-radius:4px;}QPushButton:hover{border-color:#88C0D0;}')
    layout = QVBoxLayout(browser)
    back = QPushButton('← Back to Anima')
    confirmation = QLabel()
    confirmation.setTextFormat(Qt.TextFormat.PlainText)
    origin = QLabel()
    origin.setTextFormat(Qt.TextFormat.PlainText)
    header = QHBoxLayout()
    header.addWidget(back)
    header.addStretch()
    header.addWidget(confirmation)
    header.addStretch()
    header.addWidget(origin)
    web = QWebEngineView(browser)
    remote_profile = QWebEngineProfile(web)
    web.setPage(BrowserPage(remote_profile, web))
    layout.addLayout(header)
    layout.addWidget(web)
    back.clicked.connect(browser.hide)
    web.urlChanged.connect(lambda url: origin.setText(url.host()))
    bridge.openBrowser.connect(lambda url, code: (confirmation.setText('Confirm code: '+code), web.load(QUrl(url)), browser.showFullScreen()))
    bridge.closeBrowser.connect(lambda: (browser.hide(), web.load(QUrl('about:blank')), remote_profile.cookieStore().deleteAllCookies()))
    view.load(QUrl.fromLocalFile(str(ROOT / 'index.html')))
    window.showFullScreen()
    app.aboutToQuit.connect(bridge.lock)
    return app.exec()


if __name__ == '__main__':
    sys.exit(main())
