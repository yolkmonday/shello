import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import {
  snippetDirectory,
  type SnippetCategory,
} from "../lib/snippet-directory";
import { type Recipe } from "../lib/recipe-types";

export const useRegistryStore = defineStore("registry", () => {
  const snippetCategories = ref<SnippetCategory[]>(snippetDirectory);
  const recipes = ref<Recipe[]>([]);
  const syncing = ref(false);
  const lastError = ref<string | null>(null);

  async function loadCached() {
    try {
      const [cachedSnippets, cachedRecipes] = await Promise.all([
        invoke<string | null>("registry_get", { key: "snippets" }),
        invoke<string | null>("registry_get", { key: "recipes" }),
      ]);
      if (cachedSnippets) {
        const parsed = JSON.parse(cachedSnippets) as SnippetCategory[];
        if (Array.isArray(parsed) && parsed.length > 0) {
          snippetCategories.value = parsed;
        }
      }
      if (cachedRecipes) {
        const parsed = JSON.parse(cachedRecipes) as Recipe[];
        if (Array.isArray(parsed) && parsed.length > 0) {
          recipes.value = parsed;
        }
      }
    } catch {
      // Use bundled defaults silently
    }
  }

  async function sync() {
    if (syncing.value) return;
    syncing.value = true;
    lastError.value = null;
    try {
      const results = await invoke<Record<string, string>>("registry_sync");
      if (results.snippets) {
        const parsed = JSON.parse(results.snippets) as SnippetCategory[];
        if (Array.isArray(parsed) && parsed.length > 0) {
          snippetCategories.value = parsed;
        }
      }
      if (results.recipes) {
        const parsed = JSON.parse(results.recipes) as Recipe[];
        if (Array.isArray(parsed) && parsed.length > 0) {
          recipes.value = parsed;
        }
      }
    } catch (e) {
      lastError.value = String(e);
    } finally {
      syncing.value = false;
    }
  }

  async function init() {
    await loadCached();
    sync();
  }

  return { snippetCategories, recipes, syncing, lastError, init, sync };
});
