#!/bin/sh
# Simple wishlist generator - finds files matching queries in a wishlist file
# Usage: ./simple_wishlist.sh wishlist.txt

syncweb find -tf -eZIM -S-10M
syncweb find -tf -S+1G toast
./simple_wishlist.sh simple_wishlist.txt
