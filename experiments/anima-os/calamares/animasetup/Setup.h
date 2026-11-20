#pragma once
#include "DllMacro.h"
#include "viewpages/ViewStep.h"
#include "utils/PluginFactory.h"
#include <QJsonArray>
#include <QWidget>
#include <QVBoxLayout>
#include <QCheckBox>
#include <QComboBox>
#include <QLabel>
class PLUGINDLLEXPORT Setup : public Calamares::ViewStep {
    Q_OBJECT
public:
    explicit Setup(QObject* parent = nullptr);
    QString prettyName() const override;
    QWidget* widget() override { return page; }
    bool isNextEnabled() const override;
    bool isBackEnabled() const override { return mode != "Welcome" && mode != "Ready"; }
    bool isAtBeginning() const override { return true; }
    bool isAtEnd() const override { return true; }
    Calamares::JobList jobs() const override { return {}; }
    void setConfigurationMap(const QVariantMap&) override;
    void onActivate() override;
    void onLeave() override;
private:
    QString mode;
    QWidget* page = nullptr;
    QVBoxLayout* layout = nullptr;
    QJsonArray disks;
    QComboBox* systemDisk = nullptr;
    QList<QCheckBox*> dataDisks;
    QCheckBox* confirmation = nullptr;
    QLabel* review = nullptr;
    bool failed = false;
    void storage();
    void selectionChanged();
    void refreshReview();
};
QLabel* description(const QString& text, QWidget* parent);
QWidget* networkPage();
QWidget* heroPage(const QString& title, const QString& text);
CALAMARES_PLUGIN_FACTORY_DECLARATION(SetupFactory)
