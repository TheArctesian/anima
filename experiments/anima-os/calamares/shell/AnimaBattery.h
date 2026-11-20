#pragma once
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QPainter>
#include <QProcess>
#include <QTimer>

class AnimaBattery : public QWidget
{
    QProcess process;
    QHBoxLayout* row;
    void refresh()
    {
        while (auto* item = row->takeAt(0)) { delete item->widget(); delete item; }
        if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0) return;
        const auto batteries = QJsonDocument::fromJson(process.readAllStandardOutput()).array();
        for (const auto& entry : batteries) {
            const auto battery = entry.toObject();
            const auto value = battery.value("percent");
            const double percent = value.toDouble();
            const auto state = battery.value("state").toString();
            auto* icon = new QLabel(this);
            const auto label = battery.value("name").toString() + ": "
                + (value.isNull() ? "charge unknown" : QString::number(qRound(percent)) + "%") + " · " + state;
            icon->setToolTip(label); icon->setAccessibleName(label);
            QPixmap picture(64,40); picture.setDevicePixelRatio(2); picture.fill(Qt::transparent);
            QPainter painter(&picture);
            const QColor color(!value.isNull() && percent <= 15 && state == "Discharging" ? "#D99BA1" : "#E6E7E9");
            painter.setRenderHint(QPainter::Antialiasing);
            painter.setPen(QPen(color,1.5)); painter.drawRoundedRect(QRectF(1,3,26,14),2,2); painter.drawLine(29,7,29,13);
            if (value.isNull()) painter.drawText(QRectF(4,3,20,14),Qt::AlignCenter,"?");
            else painter.fillRect(QRectF(4,6,20*percent/100,8),color);
            if (state == "Charging") {
                QPolygonF bolt; bolt << QPointF(17,1) << QPointF(10,11) << QPointF(15,11)
                    << QPointF(14,19) << QPointF(22,8) << QPointF(16,8);
                painter.setBrush(QColor("#1B1D21")); painter.drawPolygon(bolt);
            }
            painter.end(); icon->setPixmap(picture); row->addWidget(icon);
        }
    }
public:
    explicit AnimaBattery(QWidget* parent) : QWidget(parent), row(new QHBoxLayout(this))
    {
        row->setContentsMargins(0,0,0,0); row->setSpacing(8);
        connect(&process,qOverload<int,QProcess::ExitStatus>(&QProcess::finished),this,[this]{ refresh(); });
        auto* timer = new QTimer(this);
        const auto poll = [this] { if (process.state() == QProcess::NotRunning) process.start("@python@",{"@readings@"}); };
        connect(timer,&QTimer::timeout,this,poll); timer->start(5000); poll();
    }
};
