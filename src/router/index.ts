import { createRouter, createWebHistory } from "vue-router";
import Dashboard from "../app/presentation/dashboard.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      name: "dashboard",
      component: Dashboard,
    },
  ],
});

export default router;
