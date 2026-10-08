// E2E fixture: a page module that runs after init.js and listens for
// pixi:ready. It must still get the event.
window.__lateReady = [];
document.addEventListener("pixi:ready", (event) => window.__lateReady.push(event.target.id));
