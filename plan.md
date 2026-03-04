# Syncweb Development Plan

## Completed
- CLI Parity (Search, Devices, Folder Management)
- Comprehensive Web UI Tests (18/18 passing)
- Mountpoints Management UI (Listing, Mounting, Unmounting)
- Safe Removable Device Access (Unmounting multiple points, prioritizing fstab/udisks2)
- Automated Loop Device Tests for Mounts

## Next Steps

1.  **UI/UX Enhancements:**
    - Add a preview for Syncthing folders before adding them (listing contents of local path).
    - Improved styling for file/folder icons and layout.
2.  **Stability & Automation:**
    - Auto-unmount duplicate mountpoints on initial scan (Backend).
    - Ensure CI runs both Go and Vitest tests.
