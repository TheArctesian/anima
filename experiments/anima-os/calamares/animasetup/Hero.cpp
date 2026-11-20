#include "Setup.h"
#include <QPainter>
#include <QTimer>
#include <QElapsedTimer>
#include <QFile>
#include <cmath>
#include <QSysInfo>
#include <QRegularExpression>
#include <QJsonDocument>
#include <QJsonObject>
class Hero : public QWidget {
    QPixmap art;
    QElapsedTimer clock;
    QTimer timer;
public:
    Hero() : art("@hero@") {
        clock.start(); timer.setInterval(50);
        connect(&timer,&QTimer::timeout,this,qOverload<>(&QWidget::update)); timer.start();
    }
protected:
    void paintEvent(QPaintEvent*) override {
        QPainter painter(this); painter.setRenderHint(QPainter::SmoothPixmapTransform); painter.fillRect(rect(),QColor("#1B1D21"));
        const bool still=QFile::exists("/run/anima-reduced-motion");
        const double phase=still ? 0 : (1-std::cos(clock.elapsed()/16000.0*6.283185))/2;
        const double scale=1+0.045*phase;
        const int w=int(640*scale), h=int(520*scale);
        const double sourceWidth=art.height()*640.0/520.0;
        painter.setOpacity(0.9);
        painter.drawPixmap(QRect(45-int((w-640)/2), -int((h-520)/2)-int(4*phase), w,h),art,
                           QRect(int((art.width()-sourceWidth)/2),0,int(sourceWidth),art.height()));
        painter.setOpacity(1);
        QLinearGradient fade(0,0,640,0);
        fade.setColorAt(0,QColor("#1B1D21")); fade.setColorAt(.3,QColor(27,29,33,80)); fade.setColorAt(.9,QColor(27,29,33,20)); fade.setColorAt(1,QColor("#1B1D21"));
        painter.fillRect(QRect(0,0,740,520),fade);
        QLinearGradient bottom(0,0,0,520);
        bottom.setColorAt(0,QColor("#1B1D21")); bottom.setColorAt(.15,QColor(27,29,33,0)); bottom.setColorAt(.68,QColor(27,29,33,0)); bottom.setColorAt(.85,QColor("#1B1D21"));
        painter.fillRect(QRect(0,0,740,520),bottom);
    }
};
QWidget* heroPage(const QString& heading,const QString& body) {
    auto* page=new Hero; auto* box=new QVBoxLayout(page); box->setContentsMargins(40,40,40,32);
    box->setSpacing(12); box->addSpacing(72); auto* title=description(heading,page); title->setObjectName("animaHeading");
    if (heading == "Install ANIMA OS") {
        title->setTextFormat(Qt::RichText);
        title->setText("Install <span style='font-weight:500;color:#5E81AC'>A</span><span style='font-weight:500;color:#81A1C1'>N</span><span style='font-weight:500;color:#88C0D0'>I</span><span style='font-weight:500;color:#6FB3C2'>M</span><span style='font-weight:500;color:#8FBCBB'>A</span> OS");
    }
    box->addWidget(title);
    auto* text=description(body,page); text->setMaximumWidth(265); box->addWidget(text);
    if (heading == "Install ANIMA OS") {
        box->addSpacing(64);
        box->addWidget(description("This computer",page));
        QFile memory("/proc/meminfo"); memory.open(QIODevice::ReadOnly);
        auto match=QRegularExpression("MemTotal:\\s+(\\d+)").match(QString::fromUtf8(memory.readAll()));
        QFile inventory("/run/anima-installer-disks.json"); inventory.open(QIODevice::ReadOnly);
        int count=0;
        for(const auto& disk: QJsonDocument::fromJson(inventory.readAll()).array()) if(disk.toObject()["eligible"].toBool()) ++count;
        auto* facts=description(QString("%1 · %2 GiB RAM · %3 available drives").arg(QSysInfo::currentCpuArchitecture()).arg(match.captured(1).toDouble()/1048576.0,0,'f',1).arg(count),page);
        facts->setMaximumWidth(265); facts->setObjectName("animaMuted"); box->addWidget(facts);
    }
    box->addStretch();
    return page;
}
