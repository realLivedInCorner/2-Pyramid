import { createApp } from "vue";
import "remixicon/fonts/remixicon.css";
// 设计令牌（本项目唯一的颜色来源）。必须排在 App.vue 之前：
// App.vue 的全局样式引用这些变量，顺序反了首帧会取到空值。
import "./styles/tokens.css";
import App from "./App.vue";

createApp(App).mount("#app");
