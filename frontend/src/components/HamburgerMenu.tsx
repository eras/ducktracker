import React, { useState } from "react";
import { useAuthStore } from "../hooks/useAuthStore";
import { useProtocolStore } from "../lib/protocol";

const HamburgerMenu: React.FC = () => {
  const [isOpen, setIsOpen] = useState(false);
  const { username, clearCredentials } = useAuthStore();
  const { disconnect } = useProtocolStore();

  const handleLogout = () => {
    disconnect(); // Terminate SSE session
    clearCredentials(); // Clear persisted username and password
    setIsOpen(false); // Close the menu
  };

  // The logout button should only be visible if a user is logged in
  const isLoggedIn = !!username;

  return (
    <div className="fixed top-4 right-4 z-40">
      {/* Hamburger Icon */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex h-12 w-12 items-center justify-center rounded-full bg-green-700 p-2 text-white shadow-lg focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-opacity-75"
        aria-label="Open menu"
      >
        <svg
          className="h-6 w-6"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          xmlns="http://www.w3.org/2000/svg"
        >
          {isOpen ? (
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="2"
              d="M6 18L18 6M6 6l12 12"
            />
          ) : (
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="2"
              d="M4 6h16M4 12h16M4 18h16"
            />
          )}
        </svg>
      </button>

      {/* Menu Content */}
      <div
        className={`absolute top-full right-0 mt-2 w-48 origin-top-right transform rounded-md bg-green-800 shadow-xl transition-all duration-300 ease-in-out ${
          isOpen
            ? "scale-100 opacity-100"
            : "scale-95 opacity-0 pointer-events-none"
        }`}
      >
        <div className="py-1">
          {isLoggedIn ? (
            <button
              onClick={handleLogout}
              className="block w-full px-4 py-2 text-left text-sm text-red-300 hover:bg-green-700 hover:text-white"
            >
              Logout
            </button>
          ) : (
            <span className="block w-full px-4 py-2 text-left text-sm text-green-400">
              Not logged in
            </span>
          )}
          {/* Add more menu items here if needed */}
        </div>
      </div>
    </div>
  );
};

export default HamburgerMenu;
