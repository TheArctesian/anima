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
        setObjectName("animaSettings"); setMinimumWidth(440);
        auto* box=new QVBoxLayout(this); box->setContentsMargins(28,24,28,24); box->setSpacing(16);
        auto* heading=new QLabel("Display and recovery",this); heading->setObjectName("animaDialogHeading"); box->addWidget(heading);
        screens=new QComboBox(this); rotations=new QComboBox(this);
        QProcess query; query.start("anima-display",{"--list"}); query.waitForFinished(5000);
        for(const auto& line:QString::fromUtf8(query.readAllStandardOutput()).split('\n',Qt::SkipEmptyParts)) {
            const auto columns=line.split('\t'); if(columns.size()==2) screens->addItem(columns[0],columns[1]);
        }
        box->addWidget(new QLabel("Display",this)); box->addWidget(screens);
        for(const auto& rotation:QStringList{"normal","left","right","inverted"}) rotations->addItem(rotation,rotation);
        box->addWidget(new QLabel("Orientation",this)); box->addWidget(rotations);
        apply=new QPushButton("Apply orientation",this); box->addWidget(apply); apply->setEnabled(screens->count()>0);
        connect(apply,&QPushButton::clicked,this,[this]{applyRotation();});
        status=new QLabel("Changes revert unless you keep them.",this); status->setWordWrap(true); box->addWidget(status);
        keep=new QPushButton("Keep orientation",this); keep->setObjectName("view-button-next"); box->addWidget(keep); keep->hide();
        connect(keep,&QPushButton::clicked,this,[this]{timer.stop(); remaining=0; screens->setEnabled(true); rotations->setEnabled(true); screens->setItemData(screens->currentIndex(),rotations->currentData()); keep->hide(); apply->setEnabled(true); status->setText("Orientation kept for this live session.");});
        connect(&timer,&QTimer::timeout,this,[this]{if(remaining==1){revert();return;} --remaining;status->setText(QString("Reverting in %1 seconds…").arg(remaining));});
        auto* motion=new QCheckBox("Reduce background motion",this); motion->setChecked(QFile::exists("/run/anima-reduced-motion")); box->addWidget(motion);
        connect(motion,&QCheckBox::toggled,this,[](bool off){QFile f("/run/anima-reduced-motion");if(off) f.open(QIODevice::WriteOnly); else f.remove();});
        auto* recovery=new QPushButton("Open recovery terminal",this); box->addWidget(recovery);
        connect(recovery,&QPushButton::clicked,this,[]{QProcess::startDetached("xfce4-terminal",{"--disable-server"});});
        auto* shutdown=new QPushButton("Shut down live session",this); box->addWidget(shutdown);
        shutdown->setEnabled(Calamares::ViewManager::instance()->currentStepIndex() < 6);
        connect(shutdown,&QPushButton::clicked,this,[]{QProcess::startDetached("systemctl",{"poweroff"});});
        auto* close=new QPushButton("Done",this); close->setObjectName("view-button-next"); box->addWidget(close);
        connect(close,&QPushButton::clicked,this,&QDialog::accept);
    }
    ~AnimaSettings() override { revert(); }
};
