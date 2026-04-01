package commands

import (
	"fmt"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

// SyncwebScanCmd triggers a scan on all folders
type SyncwebScanCmd struct{}

func (c *SyncwebScanCmd) Run(g *SyncwebCmd) error {
	return g.WithSyncweb(func(s *syncweb.Syncweb) error {
		errors := s.ScanFolders()
		if len(errors) > 0 {
			for folderID, err := range errors {
				if err != nil {
					fmt.Printf("Error scanning folder %s: %v\n", folderID, err)
				} else {
					fmt.Printf("Scanned folder: %s\n", folderID)
				}
			}
		} else {
			fmt.Println("All folders scanned successfully")
		}
		return nil
	})
}
