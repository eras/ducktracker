import React, { useState, useEffect, useRef } from "react";
import { useAuthStore } from "../hooks/useAuthStore";

const LoginOverlay: React.FC = () => {
  const { isLoginVisible, setCredentials } = useAuthStore();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const passwordRef = useRef<HTMLInputElement>(null);
  const usernameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (isLoginVisible) {
      setIsSubmitting(false);
      setUsername("");
      setPassword("");
      // Delay focus to ensure the element is fully rendered and interactable
      const timer = setTimeout(() => {
        usernameRef.current?.focus();
      }, 50); // Small delay
      return () => clearTimeout(timer);
    }
  }, [isLoginVisible]);

  if (!isLoginVisible) {
    return null;
  }

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (username && password) {
      setIsSubmitting(true);
      setTimeout(() => {
        setCredentials(username, password);
      }, 300); // Wait for animation
    }
  };

  const handleUsernameKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault(); // Prevent form submission
      passwordRef.current?.focus(); // Move focus to password field
    }
  };

  const animationClass = isSubmitting
    ? "opacity-0 translate-y-full"
    : "opacity-100 translate-y-0";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm transition-all duration-500">
      <form onSubmit={handleSubmit} className="w-full max-w-4xl p-4">
        <div className="mb-8">
          <input
            ref={usernameRef}
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            onKeyDown={handleUsernameKeyDown}
            placeholder="_ _ _ _ _ _ _ _"
            // Removed 'text-center' from input, added 'placeholder:text-center'
            className="w-full bg-transparent font-mono text-4xl text-green-400 caret-green-400 placeholder:text-green-400/50 placeholder:text-center focus:outline-none sm:text-6xl md:text-8xl"
            autoComplete="username"
            required
          />
        </div>
        <div
          className={`transform transition-all duration-500 ease-in-out ${animationClass}`}
        >
          <input
            ref={passwordRef}
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            // Enter key on this input will naturally submit the form due to it being inside <form>
            placeholder="_ _ _ _ _ _ _ _"
            // Removed 'text-center' from input, added 'placeholder:text-center'
            className="w-full bg-transparent font-mono text-4xl text-green-400 caret-green-400 placeholder:text-green-400/50 placeholder:text-center focus:outline-none sm:text-6xl md:text-8xl"
            autoComplete="current-password"
            required
          />
        </div>
        {/* An invisible submit button to allow Enter key to submit when password field is focused */}
        <button type="submit" className="hidden" aria-hidden="true"></button>
      </form>
    </div>
  );
};

export default LoginOverlay;
