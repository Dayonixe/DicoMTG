import assert from "node:assert/strict";
import test from "node:test";
import { setupAppUpdater } from "./app-updater.js";

function setup(t, invoke, confirm = () => true, isDataBusy = () => false) {
  const elements = new Map();
  const storage = new Map();
  globalThis.document = {
    querySelector(selector) {
      const element = {
        addEventListener(event, listener) { this[event] = listener; },
        removeAttribute(name) { delete this[name]; },
      };
      elements.set(selector.replace("#app-update-", ""), element);
      return element;
    },
  };
  globalThis.window = {
    confirm,
    localStorage: {
      getItem: (key) => storage.get(key),
      setItem: (key, value) => storage.set(key, value),
    },
  };
  t.after(() => {
    delete globalThis.document;
    delete globalThis.window;
  });
  const core = { invoke, Channel: class {} };
  return { updater: setupAppUpdater(core, isDataBusy), elements, core };
}

test("detects and remembers an ignored release while allowing a manual check", async (t) => {
  let version = "3.1.1";
  const { updater, elements, core } = setup(t, async (command) => {
    if (command === "app_update_status") {
      return { currentVersion: "3.1.0", configured: true, canInstall: true };
    }
    assert.equal(command, "check_for_app_update");
    return { version, notes: "<script>Notes</script>" };
  });
  await updater.check();
  assert.equal(elements.get("current").textContent, "3.1.0");
  assert.equal(elements.get("offer").hidden, false);
  assert.equal(elements.get("notes").textContent, "<script>Notes</script>");
  elements.get("ignore").click();

  const nextSession = setupAppUpdater(core, () => false);
  await nextSession.check();
  assert.equal(elements.get("offer").hidden, true);
  await elements.get("check").click();
  assert.equal(elements.get("offer").hidden, false);
  version = "3.1.2";
  await nextSession.check();
  assert.equal(elements.get("version").textContent, "3.1.2");
});

test("requests confirmation only once data is idle and recovers from a failed install", async (t) => {
  let dataBusy = true;
  let accepted = false;
  let confirmations = 0;
  let attempts = 0;
  const { updater, elements } = setup(t, async (command, payload) => {
    if (command === "app_update_status") {
      return { currentVersion: "3.1.0", configured: true, canInstall: true };
    }
    if (command === "check_for_app_update") return { version: "3.1.1" };
    assert.equal(command, "install_app_update");
    assert.equal(payload.version, "3.1.1");
    assert.equal(updater.installing, true);
    payload.progress.onmessage({ phase: "downloading", total: 50, received: 15 });
    assert.equal(elements.get("progress").value, 15);
    attempts += 1;
    throw new Error("Réseau indisponible");
  }, () => {
    confirmations += 1;
    return accepted;
  }, () => dataBusy);

  await updater.check();
  await elements.get("install").click();
  assert.equal(confirmations, 0);
  dataBusy = false;
  await elements.get("install").click();
  assert.equal(attempts, 0);
  accepted = true;
  await elements.get("install").click();
  assert.equal(attempts, 1);
  assert.equal(updater.installing, false);
  assert.equal(elements.get("install").disabled, false);
  assert.match(elements.get("status").textContent, /Réseau indisponible/);
});
