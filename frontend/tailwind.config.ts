import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./favicon.png", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {},
  },
  plugins: [],
} satisfies Config;
