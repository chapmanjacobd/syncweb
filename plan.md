# Test Plan for Syncweb Web UI

This document outlines the strategy for adding comprehensive tests to the Syncweb Web UI, covering existing features and anticipating future CLI parity.

## 1. Test Plan: Future Functionality (CLI Parity)

These tests target features available in the CLI that should be added to the Web UI.

### Folder Management
*   **`createFolder(path)`**: Test a future UI form that calls an API to initialize a new sync folder.
*   **`deleteFolder(id)`**: Test a confirmation modal that triggers a delete API call.

### Device Management (COMPLETED)
*   **`listDevices()`**: Test fetching and rendering a list of connected devices.
*   **`addDevice(id)`**: Test a form for manually adding a device ID.
*   **`acceptDevice(id)`**: Test a "Pending Devices" notification area that allows accepting new connection requests.

### Search & Details (COMPLETED)
*   **`searchFiles(query)`**: Test a search bar that calls `/api/syncweb/find` and renders search results.
*   **`fileProperties(path)`**: Test a "Properties" action that fetches detailed file metadata via `/api/syncweb/stat`.

## Next Steps

1.  **Advanced Folder Operations:**
    - Implement and test `createFolder(path)` and `deleteFolder(id)`.
2.  **UI/UX Improvements:**
    - Add a preview for syncthing folders before adding them.
    - Implement a mountpoints list similar to the Android File Browser.
3.  **Expand Test Coverage:**
    - Ensure all `it.todo` in `web/app.test.js` are fully implemented and passing.
