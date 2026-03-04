# Test Plan for Syncweb Web UI

This document outlines the strategy for adding comprehensive tests to the Syncweb Web UI, covering existing features and anticipating future CLI parity.

## 1. Test Plan: Future Functionality (CLI Parity)

These tests target features available in the CLI that should be added to the Web UI.

### Folder Management
*   **`createFolder(path)`**: Test a future UI form that calls an API to initialize a new sync folder.
*   **`deleteFolder(id)`**: Test a confirmation modal that triggers a delete API call.

### Device Management
*   **`listDevices()`**: Test fetching and rendering a list of connected devices.
*   **`addDevice(id)`**: Test a form for manually adding a device ID.
*   **`acceptDevice(id)`**: Test a "Pending Devices" notification area that allows accepting new connection requests.

### Search & Details
*   **`fileProperties(path)`**: Test a "Properties" action that fetches detailed file metadata via `/api/syncweb/stat`.

## Next Steps

1.  **Device Management UI (COMPLETED):**
    - Create a "Devices" section in the sidebar.
    - Implement `listDevices()` in `web/app.js` to fetch from `/api/syncweb/devices`.
    - Implement `renderDevices()` to display device names and IDs in the sidebar.
    - Add a "Add Device" button that opens a prompt for a Device ID.
    - Implement `addDevice(id)` to call `/api/syncweb/devices/add`.
    - Implement tests for device listing and adding in `web/app.test.js`.
2.  **Advanced Folder Operations:**
    - Implement and test `createFolder(path)` and `deleteFolder(id)`.
3.  **UI/UX Improvements:**
    - Add a preview for syncthing folders before adding them.
    - Implement a mountpoints list similar to the Android File Browser.
4.  **Expand Test Coverage:**
    - Ensure all `it.todo` in `web/app.test.js` are fully implemented and passing.
