import { create } from "zustand";

interface AuthState {
  username?: string;
  password?: string;
  isLoginVisible: boolean;
  setCredentials: (user: string, pass: string) => void;
  showLogin: () => void;
  clearCredentials: () => void;
}

const USER_KEY = "auth_user";
const PASS_KEY = "auth_pass";

const getStoredCredentials = (): { username?: string; password?: string } => {
  try {
    return {
      username: localStorage.getItem(USER_KEY) ?? undefined,
      password: localStorage.getItem(PASS_KEY) ?? undefined,
    };
  } catch (e) {
    console.error("Failed to load credentials from localStorage", e);
    return {};
  }
};

const saveCredentials = (user: string, pass: string) => {
  try {
    localStorage.setItem(USER_KEY, user);
    localStorage.setItem(PASS_KEY, pass);
  } catch (e) {
    console.error("Failed to save credentials to localStorage", e);
  }
};

// Actually nukes everything else as well
const clearStoredCredentials = () => {
  try {
    localStorage.clear(); // Clears all items for this domain
  } catch (e) {
    console.error("Failed to clear localStorage", e);
  }
};

export const useAuthStore = create<AuthState>((set) => ({
  ...getStoredCredentials(),
  isLoginVisible: false,

  setCredentials: (user: string, pass: string) => {
    saveCredentials(user, pass);
    set({ username: user, password: pass, isLoginVisible: false });
  },

  showLogin: () => set({ isLoginVisible: true }),

  clearCredentials: () => {
    clearStoredCredentials();
    set({ username: undefined, password: undefined });
  },
}));
