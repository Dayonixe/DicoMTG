import { setUiText, t } from "./interface-language.js";

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
    setUiText(elements.status, "Recherche de mises à jour…");
    try {
      appStatus ??= await core.invoke("app_update_status");
      elements.current.textContent = appStatus.currentVersion;
      if (!appStatus.configured) {
        setUiText(elements.status, "La mise à jour intégrée sera disponible dans une version signée.");
        return;
      }
      const update = await core.invoke("check_for_app_update");
      if (!update) {
        setUiText(elements.status, "DicoMTG est à jour.");
        return;
      }
      if (!manual && update.version === ignoredVersion) {
        setUiText(elements.status, "Version {version} ignorée.", { version: update.version });
        return;
      }
      offer = update;
      elements.version.textContent = offer.version;
      const notes = offer.notes?.trim();
      if (notes) setUiText(elements.notes, () => notes);
      else setUiText(elements.notes, "Aucune note de publication.");
      elements.offer.hidden = false;
      elements.notice.hidden = false;
      elements.install.disabled = !appStatus.canInstall;
      setUiText(elements.status, appStatus.canInstall
        ? "Une mise à jour est disponible."
        : "L’installation intégrée est disponible depuis une installation Windows de DicoMTG.");
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
      setUiText(elements.status, "Attendez la fin des opérations en cours, puis réessayez.");
      return;
    }
    if (!window.confirm(t("Installer DicoMTG {version} ? L’application va se fermer pour se mettre à jour.", { version: offer.version }))) return;

    installing = true;
    elements.check.disabled = true;
    elements.install.disabled = true;
    elements.ignore.disabled = true;
    elements.progress.removeAttribute("value");
    elements.progress.hidden = false;
    setUiText(elements.status, "Téléchargement de la mise à jour…");
    try {
      const progress = new core.Channel();
      progress.onmessage = (event) => {
        if (event.phase === "installing") {
          setUiText(elements.status, "Lancement de l’installation…");
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
    setUiText(elements.status, "Version {version} ignorée.", { version: ignoredVersion });
  });
  return {
    check,
    get installing() { return installing; },
  };
}
