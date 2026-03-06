#!/bin/sh
# Blocklist script - finds and ignores specific content
# Usage: ./syncweb-blocklist.sh

syncweb find -tf -eZIM -S-10M geology
syncweb find -tf -eZIM -S-10M fishing
cat installed_zims.txt downloaded.txt
