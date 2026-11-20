#include "Setup.h"
#include "GlobalStorage.h"
#include "JobQueue.h"
#include <QPushButton>
#include <QProcess>
#include <QFile>
#include <QScrollArea>
#include <QApplication>

QLabel* description(const QString& text, QWidget* parent) {
    auto* label = new QLabel(text, parent);
    label->setWordWrap(true);
    label->setTextFormat(Qt::PlainText);
    return label;
}
Setup::Setup(QObject* parent) : Calamares::ViewStep(parent) {
    connect(Calamares::JobQueue::instance(), &Calamares::JobQueue::failed, this, [this] { failed = true; });
}
QString Setup::prettyName() const { return mode; }
bool Setup::isNextEnabled() const {
    if (mode == "Ready") return false;
    if (mode == "Storage") return systemDisk && systemDisk->currentIndex() > 0;
    if (mode == "Review") return confirmation && confirmation->isChecked();
    return true;
}
void Setup::setConfigurationMap(const QVariantMap& config) {
    mode = config.value("page").toString();
    if (mode == "Welcome") {
        page = heroPage("Install ANIMA OS", "Set up this computer as an Anima server.");
        return;
    }
    if (mode == "Network") { page = networkPage(); return; }
    if (mode == "Ready") {
        page = heroPage("Installation complete", "Choose Restart to start Anima OS, or Shut down to remove the installation USB before starting again.\nUnlock the local interface with your chosen admin password.\nSign in to Anima when you are ready to connect this computer.");
        page->setProperty("animaCompletion",true);
        return;
    }
    page = new QWidget;
    layout = new QVBoxLayout(page);
    layout->setContentsMargins(32, 28, 32, 24);
    auto* title = description(mode == "Review" ? "Review installation" : "Storage", page);
    title->setObjectName("animaHeading"); layout->addWidget(title);
    if (mode == "Storage") storage();
    if (mode == "Review") {
        review = description("", page);
        auto* scroll = new QScrollArea(page); scroll->setWidgetResizable(true); scroll->setWidget(review);
        layout->addWidget(scroll);
        confirmation = new QCheckBox("I authorize erasing every disk marked ERASE above.", page);
        layout->addWidget(confirmation);
        connect(confirmation, &QCheckBox::toggled, this, [this](bool on) { emit nextStatusChanged(on); });
    }
}
void Setup::onActivate() {
    if (mode == "Review") refreshReview();
    if (mode == "Ready" && failed) {
        for (auto* label : page->findChildren<QLabel*>()) label->setText("Installation failed. Open recovery or use Ctrl+Alt+F2 for diagnostics. Selected disks may be partially written; do not assume the system is ready.");
    }
    emit nextStatusChanged(isNextEnabled());
}
void Setup::onLeave() {
    if (mode == "Review" && confirmation->isChecked()) {
        auto* gs = Calamares::JobQueue::instance()->globalStorage();
        gs->insert("animaAuthorizedDisks", gs->value("animaSelectedDisks"));
    }
}
CALAMARES_PLUGIN_FACTORY_DEFINITION(SetupFactory, registerPlugin<Setup>();)
