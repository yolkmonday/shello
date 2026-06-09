import { createApp } from "vue";
import { createPinia } from "pinia";
import { addCollection } from "@iconify/vue";
import mdiIcons from "@iconify-json/mdi/icons.json";
import App from "./App.vue";
import "./fonts";
import "./style.css";

// Bundle the MDI icon set offline so icons render without any network
// request (local-first; also required by our strict CSP).
addCollection(mdiIcons as Parameters<typeof addCollection>[0]);

const app = createApp(App);
app.use(createPinia());
app.mount("#app");
