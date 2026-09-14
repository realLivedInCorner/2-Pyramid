import { createApp } from "vue";
import App from "./App.vue";
import i18n from "./i18n";
import "remixicon/fonts/remixicon.css";
import "./styles/shared.css";

createApp(App).use(i18n).mount("#app");
