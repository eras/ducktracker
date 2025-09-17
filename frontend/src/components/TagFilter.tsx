import React, { useState } from "react";
import { useAppStore } from "../hooks/useStore";

const TagFilter: React.FC = () => {
  const [newTag, setNewTag] = useState("");
  const {
    tags,
    customTags,
    selectedTags,
    toggleTag,
    addCustomTag,
    removeCustomTag,
  } = useAppStore();

  const handleAddTag = (e: React.FormEvent) => {
    e.preventDefault();
    if (newTag) {
      addCustomTag(newTag);
      setNewTag("");
    }
  };

  return (
    <div className="p-4 bg-white shadow-lg rounded-xl m-4 absolute bottom-0 left-0 z-10 flex flex-col gap-4 backdrop-blur-sm bg-white/70">
      <div className="flex flex-wrap gap-2">
        {tags.size > 0 ? (
          [...tags].sort().map((tag) => {
            const isCustom = customTags.has(tag);
            const isSelected = selectedTags.has(tag);

            // Determine the dynamic classes based on the tag's state.
            let colorClasses;
            if (isCustom) {
              colorClasses = isSelected
                ? "bg-purple-500 text-white ring-purple-500"
                : "bg-purple-200 text-purple-800 hover:bg-purple-300 ring-purple-300";
            } else {
              colorClasses = isSelected
                ? "bg-blue-500 text-white ring-blue-500"
                : "bg-gray-200 text-gray-800 hover:bg-gray-300 ring-gray-300";
            }

            // Combine static and dynamic classes.
            const buttonClasses = `
              px-4 py-2 rounded-full font-semibold transition-colors duration-200
              shadow-md focus:outline-none focus:ring-2 focus:ring-opacity-50
              ${colorClasses}
            `;
            return (
              <div key={tag} className="relative group">
                <button
                  onClick={() => toggleTag(tag)}
                  className={buttonClasses}
                >
                  {tag}
                </button>
                {isCustom && (
                  <button
                    onClick={() => removeCustomTag(tag)}
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
          })
        ) : (
          <span className="text-gray-500 italic">No tags available</span>
        )}
      </div>

      <form onSubmit={handleAddTag} className="flex gap-2">
        <input
          type="text"
          value={newTag}
          onChange={(e) => setNewTag(e.target.value)}
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
    </div>
  );
};

export default TagFilter;
