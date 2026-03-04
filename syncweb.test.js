import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setupTestEnvironment } from './test-helper';

describe('Syncweb Integration', () => {
    beforeEach(async () => {
        await setupTestEnvironment();
    });

    it('clicking a syncweb folder performs a search', async () => {
        // 1. Wait for syncweb folders to be rendered
        let syncBtn;
        await vi.waitFor(() => {
            syncBtn = Array.from(document.querySelectorAll('#syncweb-list .category-btn'))
                .find(btn => btn.textContent.includes('mysync'));
            expect(syncBtn).toBeTruthy();
        }, { timeout: 2000 });

        // 2. Click the syncweb folder
        syncBtn.click();

        // 3. Verify search was performed with syncweb:// prefix
        await vi.waitFor(() => {
            const searchInput = document.getElementById('search-input');
            expect(searchInput.value).toBe('syncweb://mysync/');
            
            // Should have called /api/syncweb/ls?folder=mysync&prefix=
            const calls = global.fetch.mock.calls;
            const hasLsCall = calls.some(call => 
                call[0].includes('/api/syncweb/ls') && 
                call[0].includes('folder=mysync') &&
                call[0].includes('prefix=')
            );
            expect(hasLsCall).toBe(true);
        }, { timeout: 2000 });
    });
});
