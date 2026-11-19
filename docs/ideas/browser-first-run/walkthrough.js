const screens = [...document.querySelectorAll("[data-screen]")];
const notes = [...document.querySelectorAll("[data-note]")];

function showScreen(focusHeading) {
  const id = location.hash.slice(1);
  const active = screens.find((screen) => screen.id === (id === "google-test" ? "hub" : id)) ?? screens[0];
  for (const screen of screens) screen.hidden = screen !== active;
  for (const note of notes) note.hidden = note.dataset.note !== (id === "google-test" ? id : active.id);
  document.body.classList.toggle("native", active.classList.contains("desktop"));
  visitHubScreen(active.id);
  if (focusHeading) {
    active.querySelector("h1").focus({ preventScroll: true });
    scrollTo(0, 0);
  }
}

for (const form of document.querySelectorAll("form[data-next]")) {
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    form.reset();
    location.hash = form.dataset.next;
  });
}

const previewActions = {
  reset() {
    resetDemo(false);
    if (location.hash === "#sign-in") showScreen(true);
    else location.hash = "sign-in";
  },
  return() {
    resetDemo(true);
    if (location.hash === "#hub") showScreen(true);
    else location.hash = "hub";
  },
  ready() {
    markHubReady();
    hubElement("hub-title").focus({ preventScroll: true });
  },
  cli: beginAuthorization,
  online: () => connectMachine("cli"),
  "app-open": () => beginAppHandoff(appTarget.value),
  "app-online": () => connectMachine("app"),
};
for (const button of demoButtons) button.addEventListener("click", () => previewActions[button.dataset.demo]());

addEventListener("hashchange", () => showScreen(true));
showScreen(Boolean(location.hash));
