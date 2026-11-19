function paintMachine(row, machine) {
  const icons = {
    macOS: "carbon:laptop",
    Linux: "carbon:screen",
    "Linux x86-64": "carbon:bare-metal-server",
    "Linux ARM64": "carbon:bare-metal-server",
    iOS: "carbon:mobile",
    Android: "carbon:mobile",
  };
  const icon = row.querySelector(".machine-icon");
  icon.hidden = !icons[machine.platform];
  if (!icon.hidden) paintIcon(icon, icons[machine.platform]);
  row.querySelector("strong").textContent = machine.name;
  row.querySelector("small").textContent = machine.platform;
  row.querySelector(".machine-state").textContent = machine.online ? "Online" : "Waiting";
  row.classList.toggle("online", machine.online);
}

function appendMachine(machine) {
  const row = hubElement("machine-row").content.firstElementChild.cloneNode(true);
  paintMachine(row, machine);
  deviceList.append(row);
}

function pendingMachine() {
  const device = demo.devices.at(-1);
  return device && !device.online ? device : null;
}

function startMachine(name, platform, source) {
  const machine = pendingMachine();
  if (machine) {
    Object.assign(machine, { name, platform, source });
    paintMachine(deviceList.lastElementChild, machine);
  } else {
    const waiting = { name, platform, source, online: false };
    demo.devices.push(waiting);
    appendMachine(waiting);
  }
  saveHub();
}

function terminalMachine() {
  return demo.devices.findLast((device) => device.online && !["iOS", "Android"].includes(device.platform));
}
