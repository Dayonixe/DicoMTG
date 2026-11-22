const ignoredUpdateKey = "dicomtg.ignoredAppUpdate";

export function setupAppUpdater(core, isDataBusy) {
  const elements = Object.fromEntries(
    ["notice", "current", "check", "status", "offer", "version", "notes", "install", "ignore", "progress"]
      .map((name) => [name, document.querySelector(`#app-update-${name}`)]),
  );
  let appStatus;
  let offer;
  let checking = false;
  let installing = false;
  let ignoredVersion;
  try {
    ignoredVersion = window.localStorage.getItem(ignoredUpdateKey);
  } catch {
    // Ignoring an update still works for this session.
  }

  async function check(manual = false) {
    if (!core || checking || installing) return;
    checking = true;
    elements.check.disabled = true;
    elements.notice.hidden = true;
    elements.offer.hidden = true;
    offer = undefined;
    elements.status.textContent = "Recherche de mises à jour…";
    try {
      appStatus ??= await core.invoke("app_update_status");
      elements.current.textContent = appStatus.currentVersion;
      if (!appStatus.configured) {
        elements.status.textContent = "La mise à jour intégrée sera disponible dans une version signée.";
        return;
      }
      const update = await core.invoke("check_for_app_update");
      if (!update) {
        elements.status.textContent = "DicoMTG est à jour.";
        return;
      }
      if (!manual && update.version === ignoredVersion) {
        elements.status.textContent = `Version ${update.version} ignorée.`;
        return;
      }
      offer = update;
      elements.version.textContent = offer.version;
      elements.notes.textContent = offer.notes?.trim() || "Aucune note de publication.";
      elements.offer.hidden = false;
      elements.notice.hidden = false;
      elements.install.disabled = !appStatus.canInstall;
      elements.status.textContent = appStatus.canInstall
        ? "Une mise à jour est disponible."
        : "L’installation intégrée est disponible depuis une installation Windows de DicoMTG.";
    } catch (error) {
      elements.status.textContent = String(error);
    } finally {
      checking = false;
      elements.check.disabled = appStatus?.configured === false;
    }
  }

  async function install() {
    if (!offer || !appStatus.canInstall || installing) return;
    if (isDataBusy()) {
      elements.status.textContent = "Attendez la fin des opérations en cours, puis réessayez.";
      return;
    }
    if (!window.confirm(`Installer DicoMTG ${offer.version} ? L’application va se fermer pour se mettre à jour.`)) return;

    installing = true;
    elements.check.disabled = true;
    elements.install.disabled = true;
    elements.ignore.disabled = true;
    elements.progress.removeAttribute("value");
    elements.progress.hidden = false;
    elements.status.textContent = "Téléchargement de la mise à jour…";
    try {
      const progress = new core.Channel();
      progress.onmessage = (event) => {
        if (event.phase === "installing") {
          elements.status.textContent = "Lancement de l’installation…";
          elements.progress.removeAttribute("value");
        } else if (event.total > 0) {
          elements.progress.max = event.total;
          elements.progress.value = event.received;
        }
      };
      await core.invoke("install_app_update", { version: offer.version, progress });
    } catch (error) {
      elements.status.textContent = String(error);
    } finally {
      installing = false;
      elements.check.disabled = false;
      elements.install.disabled = false;
      elements.ignore.disabled = false;
      elements.progress.hidden = true;
    }
  }

  elements.check.addEventListener("click", () => check(true));
  elements.install.addEventListener("click", install);
  elements.ignore.addEventListener("click", () => {
    if (!offer || installing) return;
    ignoredVersion = offer.version;
    try {
      window.localStorage.setItem(ignoredUpdateKey, ignoredVersion);
    } catch {
      // The current session still remembers the choice.
    }
    offer = undefined;
    elements.offer.hidden = true;
    elements.notice.hidden = true;
    elements.status.textContent = `Version ${ignoredVersion} ignorée.`;
  });
  return {
    check,
    get installing() { return installing; },
  };
}
