import { defineStore } from "pinia";
import { ref } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export const useUpdaterStore = defineStore("updater", () => {
  const checking = ref(false);
  const available = ref(false);
  const dismissed = ref(false);
  const newVersion = ref("");
  const notes = ref("");
  const downloading = ref(false);
  const downloaded = ref(0);
  const total = ref(0);
  const error = ref("");
  // Set after a manual check that found nothing, for user feedback.
  const upToDate = ref(false);

  // The pending Update instance carries the download/install methods; keep it
  // outside reactive state so Vue does not proxy its methods.
  let pending: Update | null = null;

  async function checkForUpdate(silent = true) {
    if (checking.value || downloading.value) return;
    checking.value = true;
    error.value = "";
    upToDate.value = false;
    try {
      const update = await check();
      if (update) {
        pending = update;
        available.value = true;
        dismissed.value = false;
        newVersion.value = update.version;
        notes.value = update.body ?? "";
      } else {
        available.value = false;
        if (!silent) upToDate.value = true;
      }
    } catch (e) {
      // In dev or offline the check can fail; only surface it on manual checks.
      if (!silent) error.value = String(e);
    } finally {
      checking.value = false;
    }
  }

  async function install() {
    if (!pending || downloading.value) return;
    downloading.value = true;
    error.value = "";
    downloaded.value = 0;
    total.value = 0;
    try {
      await pending.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total.value = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded.value += event.data.chunkLength;
        }
      });
      await relaunch();
    } catch (e) {
      error.value = String(e);
      downloading.value = false;
    }
  }

  function dismiss() {
    dismissed.value = true;
  }

  return {
    checking,
    available,
    dismissed,
    newVersion,
    notes,
    downloading,
    downloaded,
    total,
    error,
    upToDate,
    checkForUpdate,
    install,
    dismiss,
  };
});
