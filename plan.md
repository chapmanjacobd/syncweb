# Test Plan for Syncweb Web UI

This document outlines the strategy for adding comprehensive tests to the Syncweb Web UI, covering existing features and anticipating future CLI parity.

## 1. Test Plan: Future Functionality (CLI Parity) - COMPLETED

These tests target features available in the CLI that have been added to the Web UI.

### Folder Management (COMPLETED)
*   **`addFolder()`**: UI form to initialize a new sync folder.
*   **`deleteFolder(id)`**: Context menu action to delete a folder.

### Device Management (COMPLETED)
*   **`listDevices()`**: Fetching and rendering a list of connected devices.
*   **`addDevice(id)`**: Form for manually adding a device ID.
*   **`acceptDevice(id)`**: Clickable pending devices in sidebar to accept them.

### Search & Details (COMPLETED)
*   **`searchFiles(query)`**: Search bar that calls `/api/syncweb/find` and renders search results.
*   **`fileProperties(path)`**: Context menu "Properties" action that fetches detailed file metadata via `/api/syncweb/stat`.

## Next Steps

1.  **UI/UX Improvements:**
    - Add a preview for syncthing folders before adding them.
    - Implement a mountpoints list similar to the Android File Browser.
2.  **Continuous Testing:**
    - Maintain 100% pass rate for `web/app.test.js` (currently 15/15 passing).
