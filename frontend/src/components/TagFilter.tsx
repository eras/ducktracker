import React, { useState } from "react";
import { useAppStore } from "../hooks/useStore";

// Helper component for rendering individual tag buttons
interface TagButtonProps {
  tag: string;
  isCustom: boolean;
  isSelected: boolean;
  onToggle: (tag: string) => void;
  onRemove?: (tag: string) => void;
  isMinimized: boolean; // Existing prop
  onUnminimize: () => void;
}

const TagButton: React.FC<TagButtonProps> = ({
  tag,
  isCustom,
  isSelected,
  onToggle,
  onRemove,
  isMinimized,
  onUnminimize, // Destructure new prop
}) => {
  let colorClasses: string;
  if (isCustom) {
    colorClasses = isSelected
      ? "bg-purple-500 text-white ring-purple-500"
      : "bg-purple-200 text-purple-800 hover:bg-purple-300 ring-purple-300";
  } else {
    colorClasses = isSelected
      ? "bg-blue-500 text-white ring-blue-500"
      : "bg-gray-200 text-gray-800 hover:bg-gray-300 ring-gray-300";
  }

  const buttonClasses: string = `
    px-4 py-2 rounded-full font-semibold transition-colors duration-200
    shadow-md focus:outline-none focus:ring-2 focus:ring-opacity-50
    ${colorClasses}
  `;

  return (
    <div className="relative group">
      <button
        onClick={(e: React.MouseEvent<HTMLButtonElement>) => {
          e.stopPropagation(); // Prevent potential parent container click
          if (isMinimized) {
            onUnminimize(); // Unminimize the filter if it's currently minimized
          } else {
            onToggle(tag);
          }
        }}
        className={buttonClasses}
      >
        {tag}
      </button>
      {isCustom && !isMinimized && onRemove && (
        <button
          onClick={(e: React.MouseEvent<HTMLButtonElement>) => {
            e.stopPropagation(); // Prevent potential parent container click
            onRemove(tag);
          }}
          className="absolute -top-1 -right-1 w-5 h-5 flex items-center justify-center
                     bg-red-500 text-white rounded-full text-xs font-bold
                     transform scale-0 group-hover:scale-100 transition-transform duration-200"
          aria-label={`Remove custom tag ${tag}`}
        >
          &times;
        </button>
      )}
    </div>
  );
};

const TagFilter: React.FC = () => {
  const [newTag, setNewTag] = useState<string>("");
  const [isMinimized, setIsMinimized] = useState<boolean>(true);

  const {
    tags,
    customTags,
    selectedTags,
    toggleTag,
    addCustomTag,
    removeCustomTag,
  } = useAppStore();

  const handleAddTag = (e: React.FormEvent<HTMLFormElement>): void => {
    e.preventDefault();
    if (newTag.trim()) {
      addCustomTag(newTag.trim());
      setNewTag("");
    }
  };

  // Determine which tags to render based on minimization state
  const tagsToRender: string[] = isMinimized
    ? [...selectedTags].sort()
    : [...new Set([...tags, ...selectedTags])].sort();

  const containerBaseClasses: string = `
    p-4 bg-white shadow-lg rounded-xl m-4 absolute bottom-0 left-0 z-10
    flex flex-col gap-4 backdrop-blur-sm bg-white/70
    transition-all duration-300 ease-in-out
  `;

  const containerDynamicClasses: string = isMinimized
    ? "w-fit max-w-[250px] min-w-[180px] cursor-pointer" // Smaller size, clickable
    : "min-w-[300px] w-fit max-w-sm h-auto"; // Full size

  return (
    <div
      className={`${containerBaseClasses} ${containerDynamicClasses}`}
      // If minimized, clicking the container expands it (in addition to TagButton click)
      onClick={isMinimized ? () => setIsMinimized(false) : undefined}
    >
      {/* Header with title and minimize/maximize button */}
      {!isMinimized ? (
        <div className="flex justify-between items-center pb-2 border-b border-gray-200">
          <h3 className="text-lg font-semibold text-gray-700">Tag Filter</h3>
          <button
            onClick={(e: React.MouseEvent<HTMLButtonElement>) => {
              e.stopPropagation(); // Prevent container's onClick from firing
              setIsMinimized(!isMinimized);
            }}
            className="p-1 rounded-full hover:bg-gray-200 transition-colors"
            aria-label={
              isMinimized ? "Maximize tag filter" : "Minimize tag filter"
            }
          >
            {isMinimized ? (
              // Maximize icon (chevron down)
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-5 w-5"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fillRule="evenodd"
                  d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                  clipRule="evenodd"
                />
              </svg>
            ) : (
              // Minimize icon (chevron up)
              <svg
                xmlns="http://www.w3.org/2000/svg"
                className="h-5 w-5"
                viewBox="0 0 20 20"
                fill="currentColor"
              >
                <path
                  fillRule="evenodd"
                  d="M14.707 12.707a1 1 0 01-1.414 0L10 9.414l-3.293 3.293a1 1 0 01-1.414-1.414l4-4a1 1 0 011.414 0l4 4a1 1 0 010 1.414z"
                  clipRule="evenodd"
                />
              </svg>
            )}
          </button>
        </div>
      ) : null}
      {/* Tags Display Area */}
      <div
        className={`flex flex-wrap gap-2 ${!isMinimized ? "max-h-60 overflow-y-auto" : ""}`}
      >
        {tagsToRender.length > 0 ? (
          tagsToRender.map((tag: string) => (
            <TagButton
              key={tag}
              tag={tag}
              isCustom={customTags.has(tag)}
              isSelected={selectedTags.has(tag)}
              onToggle={toggleTag}
              onRemove={removeCustomTag}
              isMinimized={isMinimized}
              onUnminimize={() => setIsMinimized(false)} // Pass callback to unminimize
            />
          ))
        ) : (
          // Empty state messages based on minimization state
          <span className="text-gray-500 italic">
            {!isMinimized && "No tags available"}
            {isMinimized && "No tags selected"}
          </span>
        )}
      </div>

      {/* Add Tag Form (only visible when not minimized) */}
      {!isMinimized && (
        <form
          onSubmit={handleAddTag}
          className="flex gap-2 pt-2 border-t border-gray-200"
        >
          <input
            type="text"
            value={newTag}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              setNewTag(e.target.value)
            }
            placeholder="Add a new private tag"
            className="flex-grow px-4 py-2 rounded-full border border-gray-300 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <button
            type="submit"
            className="px-4 py-2 rounded-full font-semibold shadow-md
                     bg-purple-500 text-white
                     hover:bg-purple-800 transition-colors duration-200"
          >
            Add
          </button>
        </form>
      )}
    </div>
  );
};

export default TagFilter;
