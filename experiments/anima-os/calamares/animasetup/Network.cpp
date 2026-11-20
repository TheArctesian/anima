#include "Setup.h"
#include <QProcess>
#include <QPushButton>
#include <QDialog>
#include <QDialogButtonBox>
#include <QLineEdit>
#include <QTimer>
#include <QMessageBox>

static QStringList fields(const QString& line) {
    QStringList result; QString word; bool escape = false;
    for (const auto c : line) {
        if (escape) { word += c; escape = false; }
        else if (c == '\\') escape = true;
        else if (c == ':') { result.append(word); word.clear(); }
        else word += c;
    }
    result.append(word); return result;
}
static void watchStatus(QWidget* page, QLabel* status) {
    auto* statusProcess = new QProcess(page);
    auto updateStatus = [statusProcess] {
        if (statusProcess->state() == QProcess::NotRunning) statusProcess->start("nmcli", {"-t", "-f", "DEVICE,TYPE,STATE", "device", "status"});
    };
    QObject::connect(statusProcess, qOverload<int,QProcess::ExitStatus>(&QProcess::finished), page, [=](int code, QProcess::ExitStatus) {
        QStringList rows;
        for (const auto& line : QString::fromUtf8(statusProcess->readAllStandardOutput()).split('\n',Qt::SkipEmptyParts)) {
            const auto values=fields(line);
            if (values.size()==3 && (values[1]=="wifi" || values[1]=="ethernet"))
                rows.append((values[1]=="wifi" ? "Wi-Fi" : "Ethernet") + QString(" · %1 · %2").arg(values[2],values[0]));
        }
        status->setText(code != 0 ? "Network status unavailable. Open recovery for diagnostics." : rows.isEmpty() ? "No network adapters detected. You can continue offline." : rows.join("\n"));
    });
    auto* timer = new QTimer(page); timer->setInterval(4000); QObject::connect(timer,&QTimer::timeout,page,updateStatus); timer->start(); updateStatus();
}
static void scanNetworks(QWidget* page, QComboBox* wifi, QPushButton* refresh) {
    auto* scan = new QProcess(page);
    QObject::connect(refresh,&QPushButton::clicked,page,[=] {
        if (scan->state() != QProcess::NotRunning) return;
        refresh->setEnabled(false); wifi->clear(); scan->start("nmcli", {"-t", "-f", "DEVICE,BSSID,SSID,SIGNAL,SECURITY", "device", "wifi", "list", "--rescan", "yes"});
    });
    QObject::connect(scan,qOverload<int,QProcess::ExitStatus>(&QProcess::finished),page,[=](int code,QProcess::ExitStatus) {
        refresh->setEnabled(true);
        for (const auto& line : QString::fromUtf8(scan->readAllStandardOutput()).split('\n',Qt::SkipEmptyParts)) {
            const auto values=fields(line);
            if (values.size()==5) wifi->addItem(values[2]+" · "+values[3]+"% · "+values[4],QStringList{values[0],values[1]});
        }
        if (code != 0 || wifi->count()==0) wifi->addItem("No networks found. Check radio, driver or advanced settings.", "");
    });
}
static QString askPassword(QWidget* page, const QString& name, bool& accepted) {
    QDialog dialog(page,Qt::Dialog|Qt::FramelessWindowHint); dialog.setObjectName("animaNetworkDialog");
    dialog.setMinimumWidth(440); auto* form=new QVBoxLayout(&dialog); form->setContentsMargins(28,24,28,24); form->setSpacing(16);
    auto* heading=description("Connect to Wi-Fi",&dialog); heading->setObjectName("animaDialogHeading"); form->addWidget(heading);
    form->addWidget(description(name,&dialog));
    form->addWidget(description("Network password. Leave empty for an open network.",&dialog));
    auto* password=new QLineEdit(&dialog); password->setEchoMode(QLineEdit::Password); form->addWidget(password);
    auto* buttons=new QDialogButtonBox(QDialogButtonBox::Cancel|QDialogButtonBox::Ok,&dialog);
    buttons->button(QDialogButtonBox::Ok)->setText("Connect"); buttons->button(QDialogButtonBox::Ok)->setObjectName("view-button-next"); form->addWidget(buttons);
    QObject::connect(buttons,&QDialogButtonBox::accepted,&dialog,&QDialog::accept);
    QObject::connect(buttons,&QDialogButtonBox::rejected,&dialog,&QDialog::reject);
    password->setFocus(); accepted=dialog.exec()==QDialog::Accepted; return password->text();
}
static void connectionFailed(QWidget* page) {
    QMessageBox dialog(QMessageBox::NoIcon,"","Connection failed. Check the password, or use advanced settings for enterprise networks.",QMessageBox::Ok,page);
    dialog.setWindowFlags(Qt::Dialog|Qt::FramelessWindowHint); dialog.setObjectName("animaNetworkDialog");
    dialog.layout()->setContentsMargins(28,24,28,24); dialog.exec();
}
QWidget* networkPage() {
    auto* page = new QWidget; auto* box = new QVBoxLayout(page); box->setContentsMargins(32,28,32,24);
    auto* title = description("Network", page); title->setObjectName("animaHeading"); box->addWidget(title);
    box->addWidget(description("Connect now, or continue offline. System network profiles will carry over to the installed server.", page));
    auto* status = description("Checking network devices…", page); box->addWidget(status);
    auto* wifi = new QComboBox(page); box->addWidget(wifi);
    auto* refresh = new QPushButton("Scan Wi-Fi", page); box->addWidget(refresh);
    auto* connectButton = new QPushButton("Connect selected network", page); box->addWidget(connectButton);
    auto* advanced = new QPushButton("Advanced network settings…", page); box->addWidget(advanced);
    box->addWidget(description("For hidden networks, enterprise Wi-Fi or static addresses, use advanced settings. Saving a profile does not prove it connected; check the device status above.", page));
    for (auto* button : page->findChildren<QPushButton*>()) button->setMaximumWidth(260);
    wifi->setMaximumWidth(760);
    status->setObjectName("animaMuted");
    box->addStretch();
    watchStatus(page,status);
    scanNetworks(page,wifi,refresh);
    QObject::connect(advanced,&QPushButton::clicked,page,[] { QProcess::startDetached("nm-connection-editor", {}); });
    QObject::connect(connectButton,&QPushButton::clicked,page,[=] {
        const auto target = wifi->currentData().toStringList(); if (target.size()!=2) return;
        bool ok = false;
        auto secret = askPassword(page,wifi->currentText(),ok);
        if (!ok) return;
        auto* process = new QProcess(page); connectButton->setEnabled(false);
        QObject::connect(process,qOverload<int,QProcess::ExitStatus>(&QProcess::finished),page,[=](int code,QProcess::ExitStatus) {
            connectButton->setEnabled(true); process->readAllStandardOutput(); process->readAllStandardError();
            if (code != 0) connectionFailed(page);
            process->deleteLater();
        });
        process->start("nmcli", {"--wait","30","--ask","device","wifi","connect",target[1],"ifname",target[0]});
        // Password goes over a private pipe, never in arguments or installer logs.
        process->write(secret.toUtf8()+"\n"); process->closeWriteChannel(); secret.fill(QChar(0));
    });
    refresh->click(); return page;
}
