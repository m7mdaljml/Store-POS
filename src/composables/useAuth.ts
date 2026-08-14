import { useAuthStore } from "../stores/auth";

export function useAuth() {
  const auth = useAuthStore();
  return {
    user: auth.user,
    role: auth.role,
    permissions: auth.permissions,
    isAuthenticated: auth.isAuthenticated,
    can: (code: string) => auth.can(code),
    login: auth.login,
    logout: auth.logout,
    hydrate: auth.hydrate,
    verifySession: auth.verifySession,
  };
}
