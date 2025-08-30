import React from 'react';
import { useAppStore } from '../hooks/useStore';

const TagFilter: React.FC = () => {
  const { tags, selectedTags, toggleTag } = useAppStore();

  return (
    <div className="p-4 bg-white shadow-lg rounded-xl m-4 absolute top-0 left-0 z-10 flex flex-wrap gap-2 backdrop-blur-sm bg-white/70">
      {tags.length > 0 ? (
        tags.map((tag) => (
          <button
            key={tag}
            onClick={() => toggleTag(tag)}
            className={`
              px-4 py-2 rounded-full font-semibold transition-colors duration-200
              shadow-md focus:outline-none focus:ring-2 focus:ring-opacity-50
              ${selectedTags.has(tag) 
                ? 'bg-blue-500 text-white ring-blue-500' 
                : 'bg-gray-200 text-gray-800 hover:bg-gray-300 ring-gray-300'}
            `}
          >
            {tag}
          </button>
        ))
      ) : (
        <span className="text-gray-500 italic">No tags available</span>
      )}
    </div>
  );
};

export default TagFilter;