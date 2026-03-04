# Syncweb Development Plan

## Next Steps

1.  **Raw Configuration Editor:**
    - Implement `/api/syncweb/config` to GET/POST raw Syncthing configuration (XML or JSON).
    - Add a "Config" button in the sidebar to open a raw text editor.
2.  **Advanced File Operations:**
    - Implementation for copying files (`/api/file/copy`).
    - Bulk file operations (delete, move) in the UI using checkboxes.
3.  **Authentication & Security:**
    - Persistent session token (using local storage).
    - Optional password protection for the web UI.
4.  **UI/UX Enhancements:**
    - Drag-and-drop improvements (visual feedback for valid targets).
    - Folder size calculation in the UI.
    - Sorting options for file list (name, size, date).
