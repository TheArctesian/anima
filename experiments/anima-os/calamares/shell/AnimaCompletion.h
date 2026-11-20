#pragma once
#include "ViewManager.h"
#include "viewpages/ViewStep.h"
#include <QBoxLayout>
#include <QLabel>
#include <QProcess>
#include <QPushButton>
#include <QWidget>

class AnimaCompletion : public QWidget {
    QPushButton* restart;
    QPushButton* shutdown;
    QLabel* status;
    QProcess process;
    void request(const QString& action) {
        restart->setEnabled(false); shutdown->setEnabled(false);
        status->setText(action=="reboot" ? "Restarting…" : "Shutting down…");
        process.start("systemctl",{action});
    }
    void failed() {
        status->setText("Power action failed. Open recovery to try again.");
        restart->setEnabled(true); shutdown->setEnabled(true);
    }
public:
    explicit AnimaCompletion(QWidget* parent) : QWidget(parent) {
        auto* row=new QHBoxLayout(this); row->setContentsMargins(0,0,0,0); row->setSpacing(12);
        status=new QLabel(this); status->setWordWrap(true); status->setMaximumWidth(320); row->addWidget(status);
        shutdown=new QPushButton(QIcon("@powerIcon@"),"Shut down",this);
        restart=new QPushButton(QIcon("@restartIcon@"),"Restart",this); restart->setObjectName("animaRestart");
        row->addWidget(shutdown); row->addWidget(restart);
        connect(shutdown,&QPushButton::clicked,this,[this]{request("poweroff");});
        connect(restart,&QPushButton::clicked,this,[this]{request("reboot");});
        connect(&process,&QProcess::errorOccurred,this,[this]{failed();});
        connect(&process,qOverload<int,QProcess::ExitStatus>(&QProcess::finished),this,[this](int code,QProcess::ExitStatus state){
            if (code!=0 || state!=QProcess::NormalExit) failed();
        });
    }
};

inline void addCompletionActions(QWidget* navigation,QBoxLayout* layout,Calamares::ViewManager* views) {
    auto* back=navigation->findChild<QPushButton*>("view-button-back");
    auto* next=navigation->findChild<QPushButton*>("view-button-next");
    auto* power=new AnimaCompletion(navigation); layout->addWidget(power);
    const auto update=[=]{
        auto* step=views->currentStep();
        const bool complete=step && step->widget() && step->widget()->property("animaCompletion").toBool();
        power->setVisible(complete);
        back->setVisible(!complete && views->backAndNextVisible());
        next->setVisible(!complete && views->backAndNextVisible());
    };
    QObject::connect(views,&Calamares::ViewManager::currentStepChanged,navigation,update);
    QObject::connect(views,&Calamares::ViewManager::backAndNextVisibleChanged,navigation,[update](bool){update();});
    update();
}
