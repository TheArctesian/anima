#include "Setup.h"
#include "GlobalStorage.h"
#include "JobQueue.h"
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QScrollArea>
#include <QHBoxLayout>

static QString diskText(const QJsonObject& disk) {
    return QString("%1 · %2 GiB · Serial: %3 · %4")
        .arg(disk["model"].toString().trimmed())
        .arg(disk["size"].toVariant().toLongLong() / (1024*1024*1024))
        .arg(disk["serial"].toString().isEmpty() ? "not reported" : disk["serial"].toString())
        .arg(disk["path"].toString());
}
void Setup::storage() {
    QFile file("/run/anima-installer-disks.json");
    if (file.open(QIODevice::ReadOnly)) disks = QJsonDocument::fromJson(file.readAll()).array();
    layout->addWidget(description("Choose an installation disk and any additional data drives.", page));
    layout->addSpacing(12);
    layout->addWidget(description("Install Anima OS on",page));
    systemDisk = new QComboBox(page); systemDisk->setObjectName("animaSystemDisk");
    systemDisk->addItem("Choose an installation disk…", "");
    for (const auto& value : disks) {
        auto disk = value.toObject();
        if (disk["eligible"].toBool()) systemDisk->addItem(diskText(disk), disk["path"].toString());
    }
    systemDisk->setMaximumWidth(760); layout->addWidget(systemDisk);
    layout->addWidget(description("This disk will be erased. Its remaining space is available for workloads.",page));
    layout->addSpacing(16);
    layout->addWidget(description("Additional data drives — optional", page));
    auto* scroll = new QScrollArea(page); auto* rows = new QWidget(scroll); auto* list = new QVBoxLayout(rows);
    scroll->setWidgetResizable(true); scroll->setWidget(rows); layout->addWidget(scroll);
    for (const auto& value : disks) {
        auto disk = value.toObject();
        if (!disk["eligible"].toBool()) { if(disk["type"].toString() == "disk" || disk["type"].toString() == "rom") list->addWidget(description("Excluded — " + diskText(disk), rows)); continue; }
        auto* row = new QWidget(rows); row->setObjectName("animaDiskRow");
        auto* columns = new QHBoxLayout(row); columns->setContentsMargins(0,12,0,12);
        auto* check = new QCheckBox(row); columns->addWidget(check);
        columns->addWidget(description(diskText(disk),row),1);
        auto* action = description("Leave untouched",row); action->setObjectName("diskAction"); columns->addWidget(action);
        list->addWidget(row);
        check->setProperty("disk", disk["path"].toString());
        check->setProperty("label", diskText(disk));
        dataDisks.append(check);
        connect(check, &QCheckBox::toggled, this, [this] { selectionChanged(); });
    }
    list->addStretch();
    layout->addWidget(description("Added drives become separate data volumes, not a RAID array. Unselected drives stay untouched. Disk encryption is not available in this preview.",page));
    connect(systemDisk, qOverload<int>(&QComboBox::currentIndexChanged), this, [this] { selectionChanged(); });
    selectionChanged();
}
void Setup::selectionChanged() {
    QVariantList selected;
    const auto system = systemDisk->currentData().toString();
    if (!system.isEmpty()) selected.append(system);
    for (auto* check : dataDisks) {
        const auto path = check->property("disk").toString();
        const bool isSystem = path == system;
        if (isSystem) { QSignalBlocker blocker(check); check->setChecked(false); }
        check->setEnabled(!isSystem);
        check->parentWidget()->setVisible(!isSystem);
        auto* action=check->parentWidget()->findChild<QLabel*>("diskAction");
        action->setText(check->isChecked() ? "Erase & add" : "Leave untouched");
        action->setStyleSheet(check->isChecked() ? "color:#CA888F" : "color:#A4A8B0");
        if (check->isChecked()) selected.append(path);
    }
    auto* gs = Calamares::JobQueue::instance()->globalStorage();
    gs->insert("animaSelectedDisks", selected);
    gs->remove("animaAuthorizedDisks");
    emit nextStatusChanged(isNextEnabled());
}
void Setup::refreshReview() {
    auto* gs = Calamares::JobQueue::instance()->globalStorage();
    const auto selected = gs->value("animaSelectedDisks").toList();
    QString text = QString("<p>%1 disks will be erased. Check the model and serial of each.</p><table width='100%' cellspacing='12'>").arg(selected.size());
    QFile file("/run/anima-installer-disks.json");
    if (file.open(QIODevice::ReadOnly)) disks = QJsonDocument::fromJson(file.readAll()).array();
    for (const auto& value : disks) {
        const auto disk = value.toObject(); const auto path = disk["path"].toString();
        QString role = "Untouched";
        if (selected.contains(path)) role = selected.first().toString() == path ? "ERASE — Install Anima OS" : "ERASE — Separate data volume";
        if (disk["type"].toString() != "disk" && disk["type"].toString() != "rom") continue;
        text += "<tr><td><b>" + role.toHtmlEscaped() + "</b><br>" + diskText(disk).toHtmlEscaped() + "</td></tr>";
    }
    text += "</table><p>Account: admin<br>Server preview: local account and Podman; hub-driven setup is not included.<br>Encryption: off (not available in this preview)<br>Keyboard: " + gs->value("keyboardLayout").toString().toHtmlEscaped() + "</p>";
    review->setTextFormat(Qt::RichText); review->setText(text); confirmation->setChecked(false); gs->remove("animaAuthorizedDisks");
}
