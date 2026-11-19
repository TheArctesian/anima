const hubElement = (id) => document.getElementById(id);
const deviceList = hubElement("machine-list");
const motionPreference = matchMedia("(prefers-reduced-motion: reduce)");
const demoButtons = [...document.querySelectorAll("[data-demo]")];
const pathButtons = [...document.querySelectorAll("[data-path]")];
const cliTarget = hubElement("cli-target");
const appTarget = hubElement("app-target");
const appPlatforms = {
  mac: { platform: "macOS", device: "MacBook Pro", icon: "simple-icons:apple" },
  linux: { platform: "Linux", device: "Linux workstation", icon: "simple-icons:linux" },
  ios: { platform: "iOS", device: "iPhone", icon: "simple-icons:apple" },
  android: { platform: "Android", device: "Android phone", icon: "simple-icons:android" },
};
const savedHub = "anima-first-run-demo";
const demo = { screen: "", hub: "", path: "cli", authorization: "", setup: true, devices: [], boot: 0 };

function saveHub() {
  try {
    sessionStorage.setItem(savedHub, JSON.stringify({ hub: demo.hub, devices: demo.devices }));
  } catch {
    // The walkthrough also works without browser storage.
  }
}

function restoreHub() {
  try {
    const preview = JSON.parse(sessionStorage.getItem(savedHub));
    if (preview?.hub === "ready") demo.hub = "ready";
    if (Array.isArray(preview?.devices)) {
      demo.devices = preview.devices.filter((device) => device.online || device.source);
    }
  } catch {
    // An unavailable or expired preview starts fresh.
  }
}

function reviewControls() {
  const atHub = demo.screen === "hub";
  const canEnroll = demo.hub === "ready" && Boolean(pendingMachine());
  const visible = {
    reset: true,
    return: true,
    ready: atHub && demo.hub === "starting",
    cli: atHub && demo.path === "cli" && demo.setup && !demo.authorization,
    "app-open": atHub && demo.setup && demo.path === "app" &&
      !hubElement("share-panel").hidden && Boolean(appTarget.value),
    online: atHub && demo.setup && demo.path === "cli" && canEnroll && demo.authorization === "resumed",
    "app-online": canEnroll && pendingMachine()?.source === "app" &&
      (demo.screen === "connecting" || (atHub && demo.path === "app" && demo.setup)),
  };
  for (const button of demoButtons) button.hidden = !visible[button.dataset.demo];
}

function paintHub() {
  const ready = demo.hub === "ready";
  const online = demo.devices.filter((device) => device.online).length;
  const terminalAvailable = Boolean(terminalMachine());
  const status = hubElement("hub-status");
  status.classList.toggle("ready", ready);
  hubElement("hub-state").textContent = ready ? "Hub ready" : "Hub starting";
  hubElement("hub-devices").hidden = demo.devices.length === 0;
  hubElement("device-count").textContent = `${online} online`;
  hubElement("hub-title").textContent = online ? "Your hub" : "Connect your first device";
  const intro = hubElement("hub-intro");
  intro.hidden = online === 0 && demo.path === "app";
  intro.textContent = online
    ? terminalAvailable
      ? "Open a terminal on a connected computer, add more devices or connect your AI client."
      : "Add a computer for terminal access, or connect your AI client."
    : ready ? "Connect a computer or server to get started." : "Bring a computer or server into your hub. Start setup while your hub starts.";
  hubElement("first-use").hidden = online === 0;
  hubElement("open-terminal").hidden = !terminalAvailable;
  hubElement("open-terminal").className = demo.setup ? "quiet" : "action";
  hubElement("first-use").querySelector("[data-show-path]").hidden = demo.setup;
  const add = hubElement("add-device");
  add.textContent = demo.setup ? "Close setup" : pendingMachine() ? "Resume setup"
    : terminalAvailable ? "Add another device" : "Add a computer";
  add.setAttribute("aria-expanded", demo.setup);
  add.className = demo.setup || terminalAvailable ? "quiet" : "action";
  hubElement("setup").hidden = !demo.setup;
  refreshHandoff();
  reviewControls();
}

function markHubReady() {
  clearTimeout(demo.boot);
  if (demo.hub !== "starting") return;
  demo.hub = "ready";
  saveHub();
  paintHub();
  if (demo.screen === "hub") document.body.dataset.door = "ready";
}

function visitHubScreen(screen) {
  demo.screen = screen;
  if (["hub", "desktop", "device-code", "connecting"].includes(screen) && !demo.hub) {
    demo.hub = "starting";
    demo.boot = setTimeout(markHubReady, 2400);
  }
  const atHub = screen === "hub";
  const door = document.body.classList.contains("native") ? "off" : "signin";
  hubElement("hub-status").hidden = !atHub;
  if (!atHub) hubElement("hub-devices").hidden = true;
  document.body.classList.toggle("hub-view", atHub);
  document.body.dataset.door = atHub ? demo.hub : door;
  if (atHub) paintHub();
  else reviewControls();
}

function showPath(path, focus = false) {
  demo.path = path;
  if (focus) demo.setup = true;
  for (const button of pathButtons) {
    const active = button.dataset.path === path;
    button.setAttribute("aria-pressed", active);
    hubElement(button.getAttribute("aria-controls")).hidden = !active;
    if (active && focus) {
      hubElement("setup").hidden = false;
      button.focus();
    }
  }
  if (demo.screen === "hub") paintHub();
}

function updateInstaller() {
  const target = cliTarget.selectedOptions[0].textContent;
  hubElement("installer-sample").textContent = `# Preview installer for ${target}`;
}

function beginAppHandoff(target, deviceName = appPlatforms[target].device) {
  const app = appPlatforms[target];
  startMachine(deviceName, app.platform, "app");
  demo.authorization = "";
  paintHub();
  hubElement("machine-announcement").textContent = "Waiting for the app to sign in.";
}

function toggleDisclosure(button, open) {
  const panel = hubElement(button.getAttribute("aria-controls"));
  panel.hidden = !open;
  button.setAttribute("aria-expanded", open);
}

function refreshHandoff() {
  const awaitingApproval = demo.authorization === "browser";
  hubElement("cli-handoff").hidden = !demo.authorization;
  hubElement("approve-cli").hidden = !awaitingApproval;
  hubElement("handoff-title").textContent = awaitingApproval ? "Authorize Anima CLI" : "Terminal authorized";
  hubElement("handoff-instructions").textContent = awaitingApproval
    ? `Allow the terminal on ${pendingMachine()?.name ?? "your machine"} to sign in to your personal hub. Hosted CLI sign-in preview.`
    : "Return to your terminal. Setup resumes and waits for the machine to report online.";
}

function beginAuthorization() {
  const target = cliTarget.selectedOptions[0].textContent;
  startMachine(cliTarget.value === "mac" ? "MacBook Pro" : "Linux server", target, "cli");
  demo.authorization = "browser";
  paintHub();
  hubElement("approve-cli").focus();
}

function authorizeTerminal() {
  if (demo.authorization !== "browser") return;
  demo.authorization = "resumed";
  paintHub();
  hubElement("handoff-instructions").focus();
}

function connectMachine(source) {
  const machine = pendingMachine();
  if (demo.hub !== "ready" || machine?.source !== source) return;
  const row = deviceList.lastElementChild;
  machine.online = true;
  paintMachine(row, machine);
  demo.authorization = "";
  demo.setup = false;
  resetShare();
  saveHub();
  showPath(terminalMachine() ? defaultPath : "cli");
  hubElement("machine-announcement").textContent = `${machine.name} is online.`;
  if (demo.screen !== "hub") {
    hubElement("native-device").textContent = machine.name;
    location.hash = "connected";
    return;
  }
  const nextAction = terminalMachine() ? "open-terminal" : "add-device";
  hubElement(nextAction).focus({ preventScroll: true });
  if (!motionPreference.matches) {
    row.animate([{ opacity: 0.72 }, { opacity: 1 }], { duration: 340, easing: "ease-out" });
    row.querySelector(".machine-status").animate(
      [{ opacity: 0.35, transform: "translateY(3px)" }, { opacity: 1, transform: "none" }],
      { duration: 300, easing: "cubic-bezier(.18,.8,.27,1)" },
    );
  }
}

function resetDemo(returning) {
  clearTimeout(demo.boot);
  demo.hub = returning ? "ready" : "";
  demo.authorization = "";
  demo.setup = !returning;
  demo.devices = returning ? [{ name: "MacBook Pro", platform: "macOS", online: true }] : [];
  cliTarget.value = initialCliTarget;
  updateInstaller();
  deviceList.replaceChildren();
  for (const machine of demo.devices) appendMachine(machine);
  hubElement("machine-announcement").textContent = "";
  hubElement("native-device").textContent = "MacBook Pro";
  resetShare();
  for (const id of ["terminal-session", "server-panel", "vps-panel"]) {
    toggleDisclosure(document.querySelector(`[aria-controls="${id}"]`), false);
  }
  showPath(defaultPath);
  saveHub();
}

async function copyPreview(button) {
  const sample = hubElement(button.dataset.copy);
  const feedback = hubElement("copy-feedback");
  try {
    await navigator.clipboard.writeText(sample.textContent);
    feedback.textContent = "Copied to clipboard.";
  } catch {
    sample.hidden = false;
    const range = document.createRange();
    range.selectNodeContents(sample);
    getSelection().removeAllRanges();
    getSelection().addRange(range);
    feedback.textContent = "Text selected. Copy it with your browser.";
  }
  sample.closest(".setup-panel").append(feedback);
  feedback.hidden = false;
}

const platform = navigator.userAgentData?.platform || navigator.platform;
const phone = /Android/i.test(navigator.userAgent) ? "android"
  : /iPhone|iPad|iPod/i.test(navigator.userAgent) || (/Mac/.test(platform) && navigator.maxTouchPoints > 1) ? "ios" : "";
const defaultPath = phone || /Win/i.test(platform) ? "app" : "cli";
const localApp = phone || (/Mac/i.test(platform) ? "mac" : /Linux/i.test(platform) ? "linux" : "");
const tablet = phone === "ios" && (/iPad/i.test(navigator.userAgent) || /Mac/.test(platform));
if (/Linux/i.test(platform)) cliTarget.value = "linux-x64";
const initialCliTarget = cliTarget.value;

if (localApp) {
  const app = appPlatforms[localApp];
  const device = tablet ? "iPad" : phone === "ios" ? "iPhone" : "Android device";
  hubElement("app-title").textContent = phone ? `Set up Anima on this ${device}` : `Set up Anima for ${app.platform}`;
  hubElement("local-app").hidden = false;
  paintIcon(hubElement("app-platform-icon"), app.icon);
}

function resetShare() {
  appTarget.value = "";
  hubElement("share-code").hidden = true;
  hubElement("app-handoff-link").hidden = true;
  toggleDisclosure(hubElement("share-app"), false);
}

restoreHub();
demo.setup = Boolean(pendingMachine()) || !demo.devices.some((device) => device.online);
for (const machine of demo.devices) appendMachine(machine);
showPath(pendingMachine()?.source ?? (demo.devices.length && !terminalMachine() ? "cli" : defaultPath));
updateInstaller();

cliTarget.addEventListener("change", updateInstaller);
appTarget.addEventListener("change", () => {
  hubElement("copy-feedback").hidden = true;
  hubElement("app-handoff-link").hidden = true;
  hubElement("share-code").hidden = !appTarget.value;
  reviewControls();
});
hubElement("open-app").addEventListener("click", () => {
  resetShare();
  beginAppHandoff(localApp, tablet ? "iPad" : undefined);
  if (!phone) location.hash = "desktop";
});
hubElement("device-code").querySelector('a[href="#connecting"]').addEventListener("click", () => {
  if (pendingMachine()?.source !== "app") beginAppHandoff(localApp === "linux" ? "linux" : "mac");
});
hubElement("approve-cli").addEventListener("click", authorizeTerminal);
hubElement("use-server").addEventListener("click", () => {
  if (cliTarget.value === "mac") cliTarget.value = "linux-x64";
  updateInstaller();
  cliTarget.focus();
});
hubElement("open-terminal").addEventListener("click", (event) => {
  const machine = terminalMachine();
  hubElement("terminal-destination").textContent = `Session preview for ${machine.name}. A live connection is required to open a shell.`;
  toggleDisclosure(event.currentTarget, hubElement("terminal-session").hidden);
});
hubElement("add-device").addEventListener("click", () => {
  if (demo.setup) {
    demo.setup = false;
    paintHub();
    return;
  }
  showPath(pendingMachine()?.source ?? demo.path, true);
});

document.querySelector("#hub").addEventListener("click", (event) => {
  const button = event.target.closest("button");
  if (!button) return;
  hubElement("copy-feedback").hidden = true;
  if (button.dataset.path) showPath(button.dataset.path);
  if (button.dataset.showPath) showPath(button.dataset.showPath, true);
  if (button.dataset.copy) void copyPreview(button);
  if (["server-panel", "vps-panel", "share-panel"].includes(button.getAttribute("aria-controls"))) {
    toggleDisclosure(button, button.getAttribute("aria-expanded") === "false");
    reviewControls();
  }
});
