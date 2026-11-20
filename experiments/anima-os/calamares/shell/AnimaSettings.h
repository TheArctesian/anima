#pragma once
#include <QDialog>
#include "ViewManager.h"
#include <QLabel>
#include <QComboBox>
#include <QPushButton>
#include <QCheckBox>
#include <QVBoxLayout>
#include <QProcess>
#include <QFile>
#include <QTimer>
#include <QGuiApplication>
#include <QScreen>
#include <QScrollArea>
#include <QShowEvent>
#include <QWindow>

class AnimaSettings : public QDialog {
    QComboBox* screens;
    QComboBox* rotations;
    QLabel* status;
    QPushButton* apply;
    QPushButton* keep;
    QTimer timer;
    QString previous;
    QString output;
    int remaining = 0;
    void fitToScreen() {
        auto* display=screen();
        if (!display) return;
        const QRect available=display->availableGeometry().adjusted(16,16,-16,-16);
        const QSize fitted=QSize(480,700).boundedTo(available.size());
        setMaximumSize(available.size());
        resize(fitted);
        move(available.center()-QPoint(width()/2,height()/2));
    }
    void observeScreen(QScreen* display) {
        connect(display,&QScreen::geometryChanged,this,[this]{QTimer::singleShot(0,this,&AnimaSettings::fitToScreen);});
        connect(display,&QScreen::availableGeometryChanged,this,[this]{QTimer::singleShot(0,this,&AnimaSettings::fitToScreen);});
    }
    void showEvent(QShowEvent* event) override {
        QDialog::showEvent(event);
        connect(windowHandle(),&QWindow::screenChanged,this,&AnimaSettings::fitToScreen,Qt::UniqueConnection);
        QTimer::singleShot(0,this,&AnimaSettings::fitToScreen);
    }
    bool change(const QString& rotation) {
        QProcess command; command.start("anima-display", {"--set",output,rotation});
        if (!command.waitForFinished(5000)) { command.kill(); command.waitForFinished(); return false; }
        return command.exitStatus()==QProcess::NormalExit && command.exitCode()==0;
    }
    void revert() {
        if (remaining>0) {
            timer.stop(); const bool restored=change(previous); remaining=0; screens->setEnabled(true); rotations->setEnabled(true);
            status->setText(restored ? "Previous orientation restored." : "Restore failed. Use Ctrl+Alt+F2 for recovery."); keep->hide(); apply->setEnabled(true);
        }
    }
    void applyRotation() {
        output=screens->currentText(); previous=screens->currentData().toString();
        if (output.isEmpty()) return;
        if (!change(rotations->currentData().toString())) { change(previous); status->setText("Unable to change this display."); return; }
        remaining=15; screens->setEnabled(false); rotations->setEnabled(false); apply->setEnabled(false); keep->show(); timer.start(1000);
        status->setText("Keep this orientation? Reverting in 15 seconds.");
    }
public:
    explicit AnimaSettings(QWidget* parent) : QDialog(parent,Qt::Dialog|Qt::FramelessWindowHint) {
        setObjectName("animaSettings");
        auto* frame=new QVBoxLayout(this); frame->setContentsMargins(24,20,24,20); frame->setSpacing(16);
        auto* scroll=new QScrollArea(this); scroll->setWidgetResizable(true);
        scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
        auto* content=new QWidget(scroll); scroll->setWidget(content); frame->addWidget(scroll,1);
        auto* box=new QVBoxLayout(content); box->setContentsMargins(4,4,12,4); box->setSpacing(16);
        for (auto* display:QGuiApplication::screens()) observeScreen(display);
        connect(qGuiApp,&QGuiApplication::screenAdded,this,&AnimaSettings::observeScreen);
        auto* heading=new QLabel("Display and recovery",this); heading->setObjectName("animaDialogHeading"); box->addWidget(heading);
        screens=new QComboBox(this); rotations=new QComboBox(this);
        QProcess query; query.start("anima-display",{"--list"}); query.waitForFinished(5000);
        for(const auto& line:QString::fromUtf8(query.readAllStandardOutput()).split('\n',Qt::SkipEmptyParts)) {
            const auto columns=line.split('\t'); if(columns.size()==2) screens->addItem(columns[0],columns[1]);
        }
        box->addWidget(new QLabel("Display",this)); box->addWidget(screens);
        for(const auto& rotation:QStringList{"normal","left","right","inverted"}) rotations->addItem(rotation,rotation);
        box->addWidget(new QLabel("Orientation",this)); box->addWidget(rotations);
        apply=new QPushButton("Apply orientation",this); box->addWidget(apply); apply->setEnabled(screens->count()>0 && Calamares::ViewManager::instance()->currentStepIndex()<6);
        connect(apply,&QPushButton::clicked,this,[this]{applyRotation();});
        status=new QLabel(Calamares::ViewManager::instance()->currentStepIndex()<6 ? "Changes revert unless you keep them." : "Change orientation before installation or from the installed console.",this); status->setWordWrap(true); box->addWidget(status);
        keep=new QPushButton("Keep orientation",this); keep->setObjectName("view-button-next"); box->addWidget(keep); keep->hide();
        connect(keep,&QPushButton::clicked,this,[this]{
            QProcess save; save.start("anima-display",{"--save",output});
            if (!save.waitForFinished(5000) || save.exitStatus()!=QProcess::NormalExit || save.exitCode()!=0) {
                save.kill(); save.waitForFinished(); status->setText("Could not save orientation. The preview will revert."); return;
            }
            timer.stop(); remaining=0; screens->setEnabled(true); rotations->setEnabled(true); screens->setItemData(screens->currentIndex(),rotations->currentData()); keep->hide(); apply->setEnabled(true); status->setText("Orientation saved for the installed system.");});
        connect(&timer,&QTimer::timeout,this,[this]{if(remaining==1){revert();return;} --remaining;status->setText(QString("Reverting in %1 seconds…").arg(remaining));});
        auto* motion=new QCheckBox("Reduce background motion",this); motion->setChecked(QFile::exists("/run/anima-reduced-motion")); box->addWidget(motion);
        connect(motion,&QCheckBox::toggled,this,[](bool off){QFile f("/run/anima-reduced-motion");if(off) f.open(QIODevice::WriteOnly); else f.remove();});
        auto* recovery=new QPushButton("Open recovery terminal",this); box->addWidget(recovery);
        connect(recovery,&QPushButton::clicked,this,[]{QProcess::startDetached("xfce4-terminal",{"--disable-server"});});
        auto* shutdown=new QPushButton("Shut down live session",this); box->addWidget(shutdown);
        shutdown->setEnabled(Calamares::ViewManager::instance()->currentStepIndex() < 6);
        connect(shutdown,&QPushButton::clicked,this,[]{QProcess::startDetached("systemctl",{"poweroff"});});
        auto* close=new QPushButton("Done",this); close->setObjectName("view-button-next"); frame->addWidget(close);
        connect(close,&QPushButton::clicked,this,&QDialog::accept);
    }
    ~AnimaSettings() override { revert(); }
};
