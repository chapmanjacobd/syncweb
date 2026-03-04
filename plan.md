# Syncweb Development Plan

## Completed
- CLI Parity (Search, Devices, Folder Management)
- Mountpoints Management UI (Listing, Mounting, Unmounting)
- Safe Removable Device Access (Unmounting multiple points, prioritizing fstab/udisks2)
- Automated Loop Device Tests for Mounts
- Root Device Exclusion (Ensuring "/" is never unmounted or treated as duplicate)
- Folder Preview UI
- UI/UX Polishing (Lucide icons, modern styling, responsive sidebar, enhanced notifications)

## Next Steps

1.  **Backend Robustness:**
    - Auto-unmount duplicate mountpoints on initial scan.
    - Implement `/api/syncweb/config` to view/edit raw syncthing settings.
2.  **Advanced File Operations:**
    - Implementation for copying files.
    - Bulk file operations (delete, move).
3.  **Authentication & Security:**
    - Persistent session token (local storage).
    - Optional password protection for the web UI.
