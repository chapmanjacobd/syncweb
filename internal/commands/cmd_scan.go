package commands

import (
	"fmt"

	"github.com/chapmanjacobd/syncweb/internal/syncweb"
)

// Scan command examples
const scanExamples = `
Examples:
  # Scan all folders
  syncweb scan

  # Trigger rescan after adding files externally
  syncweb scan
`

// SyncwebScanCmd triggers a scan on all folders
type SyncwebScanCmd struct{}

// Help displays examples for the scan command
func (c *SyncwebScanCmd) Help() string {
	return scanExamples
}

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
