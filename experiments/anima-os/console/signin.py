"""Separate, off-record sign-in browser with compact native navigation."""
from PySide6.QtCore import Qt, QUrl
from PySide6.QtGui import QColor
from PySide6.QtWidgets import QHBoxLayout, QLabel, QPushButton, QSizePolicy, QVBoxLayout, QWidget
from PySide6.QtWebEngineCore import QWebEnginePage, QWebEngineProfile
from PySide6.QtWebEngineWidgets import QWebEngineView


class BrowserPage(QWebEnginePage):
    def acceptNavigationRequest(self, url, kind, main_frame):
        return url.scheme() == 'https' or url.toString() == 'about:blank'


class HostLabel(QLabel):
    def __init__(self):
        super().__init__()
        self.host = ''
        self.setTextFormat(Qt.TextFormat.PlainText)
        self.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
        self.setSizePolicy(QSizePolicy.Policy.Ignored, QSizePolicy.Policy.Preferred)

    def show_host(self, url):
        self.host = url.host()
        self.setToolTip(self.host)
        self.setText(self.fontMetrics().elidedText(self.host, Qt.TextElideMode.ElideMiddle, self.width()))

    def resizeEvent(self, event):
        super().resizeEvent(event)
        self.setText(self.fontMetrics().elidedText(self.host, Qt.TextElideMode.ElideMiddle, self.width()))


class SigninWindow(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowFlags(Qt.WindowType.FramelessWindowHint)
        self.setStyleSheet('QWidget{background:#1B1D21;color:#E6E7E9;font:14px "IBM Plex Sans";}QWidget#signinHeader{border-bottom:1px solid #2C3036;}QLabel{border:0;}QPushButton{padding:8px 14px;border:1px solid #353941;border-radius:4px;}QPushButton:hover,QPushButton:focus{border-color:#88C0D0;}')
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(0)
        header = QWidget(self)
        header.setObjectName('signinHeader')
        header.setFixedHeight(64)
        row = QHBoxLayout(header)
        row.setContentsMargins(24, 8, 24, 8)
        row.setSpacing(16)
        back = QPushButton('← Back to Anima', header)
        self.confirmation = QLabel(header)
        self.confirmation.setTextFormat(Qt.TextFormat.PlainText)
        self.confirmation.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.origin = HostLabel()
        row.addWidget(back)
        row.addWidget(self.confirmation, 1)
        row.addWidget(self.origin, 1)
        self.web = QWebEngineView(self)
        self.profile = QWebEngineProfile(self.web)
        page = BrowserPage(self.profile, self.web)
        page.setBackgroundColor(QColor('#1B1D21'))
        self.web.setPage(page)
        layout.addWidget(header, 0)
        layout.addWidget(self.web, 1)
        back.clicked.connect(self.hide)
        self.web.urlChanged.connect(self.origin.show_host)

    def open_signin(self, url, code):
        self.confirmation.setText('Confirm code: '+code)
        self.web.load(QUrl(url))
        self.showFullScreen()

    def clear(self):
        self.hide()
        self.web.load(QUrl('about:blank'))
        self.profile.cookieStore().deleteAllCookies()
